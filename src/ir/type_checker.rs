use error::{self, ErrorKind};
use ir::block::{Block, CallData, Expr, Statement};
use src_tag::SrcTagged;
use symbol_table::{SymbolName, SymbolTable};
use base_type::BaseType;

pub trait TypeChecking {
    fn base_type(&self) -> Option<&BaseType>;
    fn infer_types(&mut self, symbol_table: &SymbolTable) -> error::Result<()>;
    fn imply_type(&mut self, base_type: &BaseType);
    fn imply_defaults(&mut self);
    fn resolve_type(&mut self, symbol_table: &SymbolTable) -> error::Result<BaseType>;
}

impl TypeChecking for Expr {
    fn base_type(&self) -> Option<&BaseType> {
        use ir::block::Expr::*;
        match *self {
            Number(ref data) => data.value_type.as_ref(),
            Symbol(ref data) => data.value_type.as_ref(),
            BinaryOp(ref data) => data.result_type.as_ref(),
            Call(ref data) => data.base_type(),
            ArrayIndex(ref data) => data.array_type.as_ref().and_then(|at| at.underlying_type()),
        }
    }

    fn infer_types(&mut self, symbol_table: &SymbolTable) -> error::Result<()> {
        use ir::block::Expr::*;
        match *self {
            Number(_) => {}
            Symbol(ref mut data) => {
                if let Some(constant) = symbol_table.constant(data.symbol) {
                    data.value_type = Some(constant.base_type);
                } else if let Some(variable) = symbol_table.variable(data.symbol) {
                    data.value_type = Some(variable.base_type);
                } else {
                    unreachable!()
                }
            }
            BinaryOp(ref mut data) => {
                data.left.infer_types(symbol_table)?;
                data.right.infer_types(symbol_table)?;

                if !data.op.is_arithmetic() && data.left.base_type().is_none() && data.right.base_type().is_none() {
                    data.left.imply_defaults();
                    data.right.imply_defaults();
                }

                let left_type = data.left.base_type().cloned();
                let right_type = data.right.base_type().cloned();

                if left_type.is_none() && right_type.is_some() {
                    data.left.imply_type(right_type.as_ref().unwrap());
                    data.result_type = right_type;
                } else if left_type.is_some() && right_type.is_none() {
                    data.right.imply_type(left_type.as_ref().unwrap());
                    data.result_type = left_type;
                }
            }
            Call(ref mut data) => data.infer_types(symbol_table)?,
            ArrayIndex(ref mut data) => {
                data.index.infer_types(symbol_table)?;
                if data.index.base_type().is_none() {
                    data.index.imply_type(&BaseType::U16);
                }
                if let Some(constant) = symbol_table.constant(data.array) {
                    data.array_type = Some(constant.base_type);
                } else if let Some(variable) = symbol_table.variable(data.array) {
                    data.array_type = Some(variable.base_type);
                } else {
                    unreachable!()
                }
            }
        }
        Ok(())
    }

    fn imply_type(&mut self, base_type: &BaseType) {
        use ir::block::Expr::*;
        match *self {
            Number(ref mut data) => {
                if data.value_type.is_none() {
                    data.value_type = Some(base_type.clone());
                }
            }
            BinaryOp(ref mut data) => {
                if data.result_type.is_none() {
                    data.result_type = Some(base_type.clone());
                    if data.op.is_arithmetic() {
                        data.left.imply_type(base_type);
                        data.right.imply_type(base_type);
                    }
                }
            }
            Symbol(_) | Call(_) | ArrayIndex(_) => {}
        }
    }

    fn imply_defaults(&mut self) {
        if let Expr::Number(ref mut data) = *self {
            if data.value_type.is_none() {
                data.value_type = Some(BaseType::U8);
            }
        }
    }

    fn resolve_type(&mut self, symbol_table: &SymbolTable) -> error::Result<BaseType> {
        use ir::block::Expr::*;
        match *self {
            Number(ref data) => {
                match data.value_type {
                    Some(ref base_type) => Ok(base_type.clone()),
                    None => Err(
                        ErrorKind::TypeExprError(data.tag, "Can't infer type of number".into()).into(),
                    ),
                }
            }
            BinaryOp(ref mut data) => {
                let left_type = data.left.resolve_type(symbol_table)?;
                let right_type = data.right.resolve_type(symbol_table)?;
                match BaseType::choose_type(&left_type, &right_type) {
                    Some(base_type) => {
                        data.result_type = Some(base_type.clone());
                        Ok(base_type)
                    }
                    None => Err(
                        ErrorKind::TypeExprError(
                            data.tag,
                            format!(
                                "Can't perform arithmetic between {} and {}",
                                left_type,
                                right_type
                            ),
                        ).into(),
                    ),
                }
            }
            Call(ref mut data) => data.resolve_type(symbol_table),
            ArrayIndex(ref mut data) => {
                if !data.array_type.as_ref().unwrap().can_index() {
                    Err(
                        ErrorKind::TypeExprError(
                            data.tag,
                            format!("Can't index {}", data.array_type.as_ref().unwrap()),
                        ).into(),
                    )
                } else if data.index.base_type().is_none() {
                    Err(
                        ErrorKind::TypeExprError(
                            data.index.src_tag(),
                            "Can't infer type of array index".into(),
                        ).into(),
                    )
                } else {
                    Ok(
                        data.array_type.as_ref().unwrap().underlying_type().unwrap().clone(),
                    )
                }
            }
            Symbol(ref data) => Ok(data.value_type.as_ref().unwrap().clone()),
        }
    }
}

impl TypeChecking for CallData {
    fn base_type(&self) -> Option<&BaseType> {
        self.return_type.as_ref()
    }

    fn infer_types(&mut self, symbol_table: &SymbolTable) -> error::Result<()> {
        if let Some(function) = symbol_table.function_by_name(&self.function) {
            self.return_type = Some(function.read().unwrap().return_type.clone());
            let expected_arg_count = function.read().unwrap().parameters.len();
            if self.arguments.len() != expected_arg_count {
                return Err(
                    ErrorKind::ExpectedNArgumentsGotM(
                        self.tag,
                        SymbolName::clone(&self.function),
                        expected_arg_count,
                        self.arguments.len(),
                    ).into(),
                );
            }
            for (index, argument) in self.arguments.iter_mut().enumerate() {
                argument.infer_types(symbol_table)?;
                if argument.base_type().is_none() {
                    argument.imply_type(&function.read().unwrap().parameters[index].base_type);
                }
            }
        } else {
            return Err(
                ErrorKind::SymbolNotFound(self.tag, SymbolName::clone(&self.function)).into(),
            );
        }
        Ok(())
    }

    fn imply_type(&mut self, _base_type: &BaseType) {}

    fn imply_defaults(&mut self) {}

    fn resolve_type(&mut self, symbol_table: &SymbolTable) -> error::Result<BaseType> {
        if let Some(function) = symbol_table.function_by_name(&self.function) {
            for (index, argument) in self.arguments.iter_mut().enumerate() {
                let argument_type = argument.resolve_type(symbol_table)?;
                if !argument_type.can_assign_into(&function.read().unwrap().parameters[index].base_type) {
                    return Err(
                        ErrorKind::TypeExprError(
                            argument.src_tag(),
                            format!(
                                "Argument {} expected {} but got a {}",
                                index + 1,
                                function.read().unwrap().parameters[index].base_type,
                                argument_type
                            ),
                        ).into(),
                    );
                }
            }
            Ok(self.return_type.as_ref().unwrap().clone())
        } else {
            unreachable!()
        }
    }
}

impl TypeChecking for Statement {
    fn base_type(&self) -> Option<&BaseType> {
        None
    }

    fn infer_types(&mut self, symbol_table: &SymbolTable) -> error::Result<()> {
        use ir::block::Statement::*;
        match *self {
            Assign(ref mut data) => {
                data.left_value.infer_types(symbol_table)?;
                data.right_value.infer_types(symbol_table)?
            }
            Call(ref mut data) => data.infer_types(symbol_table)?,
            Conditional(ref mut data) => {
                data.condition.infer_types(symbol_table)?;
                for statement in &mut data.when_true {
                    statement.infer_types(symbol_table)?;
                }
                for statement in &mut data.when_false {
                    statement.infer_types(symbol_table)?;
                }
            }
            Return(ref mut data) => {
                if let Some(ref mut value) = data.value {
                    value.infer_types(symbol_table)?;
                }
            }
            WhileLoop(ref mut data) => {
                data.condition.infer_types(symbol_table)?;
                for statement in &mut data.body {
                    statement.infer_types(symbol_table)?;
                }
            }
            Break | GoTo(_) | InlineAsm(_) => {}
        }
        Ok(())
    }

    fn imply_type(&mut self, base_type: &BaseType) {
        use ir::block::Statement::*;
        match *self {
            Return(ref mut data) => {
                if let Some(ref mut value) = *(&mut data.value) {
                    value.imply_type(base_type);
                }
            }
            Conditional(ref mut data) => {
                for mut statement in &mut data.when_true {
                    statement.imply_type(base_type);
                }
                for mut statement in &mut data.when_false {
                    statement.imply_type(base_type);
                }
            }
            WhileLoop(ref mut data) => {
                for mut statement in &mut data.body {
                    statement.imply_type(base_type);
                }
            }
            _ => {}
        }
    }

    fn imply_defaults(&mut self) {}

    fn resolve_type(&mut self, symbol_table: &SymbolTable) -> error::Result<BaseType> {
        use ir::block::Statement::*;
        match *self {
            Assign(ref mut data) => {
                let left_type = data.left_value.resolve_type(symbol_table)?;
                data.right_value.imply_type(&left_type);

                let right_type = data.right_value.resolve_type(symbol_table)?;
                if !right_type.can_assign_into(&left_type) {
                    return Err(
                        ErrorKind::TypeExprError(
                            data.tag,
                            format!("Can't assign {} into {}", right_type, left_type),
                        ).into(),
                    );
                }
                data.value_type = Some(left_type);
            }
            Call(ref mut data) => {
                data.resolve_type(symbol_table)?;
            }
            Conditional(ref mut data) => {
                let condition_type = data.condition.resolve_type(symbol_table)?;
                if !condition_type.can_cast_into(&BaseType::Bool) {
                    return Err(
                        ErrorKind::TypeExprError(data.tag, "Condition can't evaluate to a boolean".into()).into(),
                    );
                }
                for mut statement in &mut data.when_true {
                    statement.resolve_type(symbol_table)?;
                }
                for mut statement in &mut data.when_false {
                    statement.resolve_type(symbol_table)?;
                }
            }
            Return(ref mut data) => {
                if let Some(ref mut value) = *(&mut data.value) {
                    data.value_type = Some(value.resolve_type(symbol_table)?);
                }
            }
            WhileLoop(ref mut data) => {
                let condition_type = data.condition.resolve_type(symbol_table)?;
                if !condition_type.can_cast_into(&BaseType::Bool) {
                    return Err(
                        ErrorKind::TypeExprError(data.tag, "Condition can't evaluate to a boolean".into()).into(),
                    );
                }
                for mut statement in &mut data.body {
                    statement.resolve_type(symbol_table)?;
                }
            }
            Break | GoTo(_) | InlineAsm(_) => {}
        }
        Ok(BaseType::Void)
    }
}

pub fn resolve_types(blocks: &mut Vec<Block>) -> error::Result<()> {
    for block in blocks {
        let symbol_table = &*block.symbol_table.read().unwrap();
        let return_type = block.metadata.read().unwrap().return_type.clone();
        for mut statement in &mut block.body {
            statement.infer_types(symbol_table)?;
            statement.imply_type(&return_type);
            statement.resolve_type(symbol_table)?;
        }
        // TODO: Iterate over all return statements (recursively) and verify return type
    }
    Ok(())
}
