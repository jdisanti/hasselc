use std::sync::{Arc, RwLock};
use num_traits::PrimInt;
use error::{self, ErrorKind};
use ir;
use parse::ast;
use src_tag::{SrcTag, SrcTagged};
use symbol_table::{FunctionMetadata, FunctionMetadataPtr, Location, SymbolName, SymbolTable, Variable};
use types::{AddressOrSymbol, Type, TypedValue};

pub fn generate(
    global_symbol_table: &Arc<RwLock<SymbolTable>>,
    input: &[ast::Expression],
) -> error::Result<Vec<ir::Block>> {
    let mut blocks = vec![ir::Block::new_anonymous(Arc::clone(global_symbol_table))];

    for ast_expr in input {
        match *ast_expr {
            ast::Expression::DeclareFunction(ref data) => {
                let location = if blocks.last_mut().unwrap().is_empty_anonymous() {
                    let old_block = blocks.pop().unwrap();
                    old_block.location
                } else {
                    None
                };

                let symbol_name = SymbolName::clone(&data.name);
                let metadata = Arc::new(RwLock::new(FunctionMetadata {
                    name: Arc::clone(&data.name),
                    location: location,
                    parameters: data.parameters.clone(),
                    return_type: data.return_type,
                    frame_size: 127, // 127 is an intentional non-sensical value
                }));

                let optional_function_ref = global_symbol_table.write().unwrap().insert_function(
                    SymbolName::clone(&symbol_name),
                    FunctionMetadataPtr::clone(&metadata),
                );
                if let Some(function_ref) = optional_function_ref {
                    let mut function = ir::Block::new_named(
                        data.tag,
                        function_ref,
                        Arc::clone(global_symbol_table),
                        location,
                        FunctionMetadataPtr::clone(&metadata),
                    )?;

                    let body_ir = generate_statement_irs(&mut *function.symbol_table.write().unwrap(), &data.body)?;
                    function.body.extend(body_ir);
                    blocks.push(function);
                } else {
                    return Err(ErrorKind::DuplicateSymbol(data.tag, Arc::clone(&symbol_name)).into());
                }
            }
            ast::Expression::Org(ref data) => {
                if data.address < 0x200 || data.address > 0xFFFF {
                    return Err(ErrorKind::OrgOutOfRange(data.tag).into());
                }
                blocks.last_mut().unwrap().location = Some(Location::Global(data.address as u16));
            }
            ast::Expression::Comment => {}
            ast::Expression::Error => unreachable!("error"),
            ast::Expression::BinaryOp { .. } => unreachable!("binary_op"),
            ast::Expression::Number(_) => unreachable!("number"),
            ast::Expression::Name(_) => unreachable!("name"),
            _ => {
                let stmt = generate_statement_ir(&mut *global_symbol_table.write().unwrap(), ast_expr)?;
                blocks.last_mut().unwrap().body.extend(stmt);
            }
        }
    }

    ir::type_checker::resolve_types(&mut blocks)?;
    Ok(blocks)
}

fn generate_statement_irs(
    symbol_table: &mut SymbolTable,
    input: &[ast::Expression],
) -> error::Result<Vec<ir::Statement>> {
    let mut statements: Vec<ir::Statement> = vec![];

    for ast_expr in input {
        statements.extend(generate_statement_ir(symbol_table, ast_expr)?);
    }

    Ok(statements)
}

fn generate_statement_ir(symbol_table: &mut SymbolTable, input: &ast::Expression) -> error::Result<Vec<ir::Statement>> {
    let mut statements: Vec<ir::Statement> = vec![];
    match *input {
        ast::Expression::Assignment(ref data) => {
            let left_value = generate_expression(symbol_table, &data.left_value)?;
            let right_value = generate_expression(symbol_table, &data.right_value)?;
            statements.push(ir::Statement::Assign(ir::AssignData::new(
                data.tag,
                Type::Unresolved,
                left_value,
                right_value,
            )));
        }
        ast::Expression::Break => {
            unimplemented!("ir_gen: break");
        }
        ast::Expression::CallFunction(ref data) => {
            let stmt = ir::Statement::Call(ir::CallData::new(
                data.tag,
                SymbolName::clone(&data.name),
                generate_expressions(symbol_table, &data.arguments)?,
                Type::Unresolved,
            ));
            statements.push(stmt);
        }
        ast::Expression::Conditional(ref data) => {
            let condition = generate_expression(symbol_table, &data.condition)?;
            let when_true = generate_statement_irs(symbol_table, &data.when_true)?;
            let when_false = generate_statement_irs(symbol_table, &data.when_false)?;
            statements.push(ir::Statement::Conditional(ir::ConditionalData::new(
                data.tag,
                condition,
                when_true,
                when_false,
            )));
        }
        ast::Expression::DeclareConst(ref data) => {
            let symbol_name = SymbolName::clone(&data.name_type.name);
            let value = constant_eval(symbol_table, data.name_type.type_name, &*data.value)?;
            if symbol_table
                .insert_constant(SymbolName::clone(&symbol_name), value)
                .is_none()
            {
                return Err(ErrorKind::DuplicateSymbol(data.tag, SymbolName::clone(&data.name_type.name)).into());
            }
        }
        ast::Expression::DeclareRegister(ref data) => {
            if data.location < 0 || data.location > 0xFFFF {
                return Err(ErrorKind::OutOfBounds(data.tag, data.location as isize, 0, 0xFFFF).into());
            }
            let symbol_name = SymbolName::clone(&data.name_type.name);
            let variable = Variable::new(
                data.name_type.type_name,
                Location::Global(data.location as u16),
            );
            if symbol_table
                .insert_variable(symbol_name, variable)
                .is_none()
            {
                return Err(ErrorKind::DuplicateSymbol(data.tag, SymbolName::clone(&data.name_type.name)).into());
            }
        }
        ast::Expression::DeclareVariable(ref data) => {
            let symbol_name = SymbolName::clone(&data.name_type.name);
            let next_location = symbol_table.next_frame_offset(data.name_type.type_name.size());
            let variable = Variable::new(
                data.name_type.type_name,
                Location::FrameOffset(next_location as i8),
            );
            if let Some(symbol_ref) = symbol_table.insert_variable(SymbolName::clone(&symbol_name), variable) {
                let assignment = ir::Statement::Assign(ir::AssignData::new(
                    data.tag,
                    Type::Unresolved,
                    ir::Expr::Symbol(ir::SymbolData::new(data.tag, symbol_ref, Type::Unresolved)),
                    generate_expression(symbol_table, &data.value)?,
                ));
                statements.push(assignment);
            } else {
                return Err(ErrorKind::DuplicateSymbol(data.tag, SymbolName::clone(&data.name_type.name)).into());
            }
        }
        ast::Expression::GoTo(ref data) => {
            statements.push(ir::Statement::GoTo(
                ir::GoToData::new(data.tag, Arc::clone(&data.destination)),
            ));
        }
        ast::Expression::InlineAsm(ref data) => {
            statements.push(ir::Statement::InlineAsm(
                ir::InlineAsmData::new(data.tag, Arc::clone(&data.asm)),
            ));
        }
        ast::Expression::Return(ref data) => {
            let value = match data.value {
                Some(ref val) => Some(generate_expression(symbol_table, val)?),
                None => None,
            };
            statements.push(ir::Statement::Return(
                ir::ReturnData::new(data.tag, Type::Unresolved, value),
            ));
        }
        ast::Expression::WhileLoop(ref data) => {
            let condition = generate_expression(symbol_table, &data.condition)?;
            let body = generate_statement_irs(symbol_table, &data.body)?;
            statements.push(ir::Statement::WhileLoop(
                ir::WhileLoopData::new(data.tag, condition, body),
            ));
        }
        ast::Expression::Comment => {}
        ast::Expression::ArrayIndex(_) => unreachable!("array_index"),
        ast::Expression::BinaryOp { .. } => unreachable!("binary_op"),
        ast::Expression::DeclareFunction { .. } => unreachable!("declare_function"),
        ast::Expression::Error => unreachable!("error"),
        ast::Expression::Name(_) => unreachable!("name"),
        ast::Expression::Number(_) => unreachable!("number"),
        ast::Expression::Org { .. } => unreachable!("org"),
        ast::Expression::Text(_) => unreachable!("text"),
    }

    Ok(statements)
}

fn constant_eval(
    symbol_table: &mut SymbolTable,
    type_name: Type,
    input: &ast::Expression,
) -> error::Result<TypedValue> {
    match *input {
        ast::Expression::BinaryOp(ref data) => {
            let left = constant_eval(symbol_table, type_name, &*data.left)?;
            let right = constant_eval(symbol_table, type_name, &*data.right)?;
            match type_name {
                Type::U8 => Ok(TypedValue::U8(constant_eval_binop(
                    data.tag,
                    data.op,
                    left.as_u8(),
                    right.as_u8(),
                )?)),
                Type::U16 => Ok(TypedValue::U16(constant_eval_binop(
                    data.tag,
                    data.op,
                    left.as_u16(),
                    right.as_u16(),
                )?)),
                Type::ArrayU8 => unimplemented!(),
                Type::Void => Err(ErrorKind::ConstCantBeVoid(data.tag).into()),
                Type::Unresolved => unreachable!(),
            }
        }
        ast::Expression::Name(ref data) => match symbol_table.constant_by_name(&data.name) {
            Some(constant) => Ok(constant),
            None => Err(ErrorKind::SymbolNotFound(data.tag, SymbolName::clone(&data.name)).into()),
        },
        ast::Expression::Number(ref data) => constant_eval_number(type_name, data),
        ast::Expression::Text(ref data) => if type_name != Type::ArrayU8 {
            Err(ErrorKind::TypeError(data.tag, Type::ArrayU8, type_name).into())
        } else {
            let symbol_ref = symbol_table.insert_text(Arc::clone(&data.value));
            Ok(TypedValue::ArrayU8(AddressOrSymbol::Symbol(symbol_ref)))
        },
        _ => Err(ErrorKind::ConstEvaluationFailed(input.src_tag()).into()),
    }
}

fn constant_eval_number(type_name: Type, input: &ast::NumberData) -> error::Result<TypedValue> {
    match type_name {
        Type::U8 => {
            let unsigned_val = input.value as usize;
            if unsigned_val > 0xFF {
                Err(ErrorKind::OutOfBounds(input.tag, input.value as isize, 0, 0xFF).into())
            } else {
                Ok(TypedValue::U8(input.value as u8))
            }
        }
        Type::U16 => {
            let unsigned_val = input.value as usize;
            if unsigned_val > 0xFFFF {
                Err(ErrorKind::OutOfBounds(input.tag, input.value as isize, 0, 0xFFFF).into())
            } else {
                Ok(TypedValue::U16(input.value as u16))
            }
        }
        Type::ArrayU8 => {
            let unsigned_val = input.value as usize;
            if unsigned_val > 0xFFFF {
                Err(ErrorKind::OutOfBounds(input.tag, input.value as isize, 0, 0xFFFF).into())
            } else {
                Ok(TypedValue::ArrayU8(
                    AddressOrSymbol::Address(input.value as u16),
                ))
            }
        }
        Type::Void => Err(ErrorKind::ConstCantBeVoid(input.tag).into()),
        Type::Unresolved => unreachable!(),
    }
}

fn constant_eval_binop<N: PrimInt>(tag: SrcTag, op: ast::BinaryOperator, left: N, right: N) -> error::Result<N> {
    use parse::ast::BinaryOperator::*;
    let result: Option<N> = match op {
        Add => left.checked_add(&right),
        Sub => left.checked_sub(&right),
        Mul => left.checked_mul(&right),
        Div => left.checked_div(&right),
        LessThan => if left < right {
            N::from(1)
        } else {
            N::from(0)
        },
        GreaterThan => if left > right {
            N::from(1)
        } else {
            N::from(0)
        },
        LessThanEqual => if left <= right {
            N::from(1)
        } else {
            N::from(0)
        },
        GreaterThanEqual => if left >= right {
            N::from(1)
        } else {
            N::from(0)
        },
        Equal => if left == right {
            N::from(1)
        } else {
            N::from(0)
        },
        NotEqual => if left != right {
            N::from(1)
        } else {
            N::from(0)
        },
    };
    match result {
        Some(val) => Ok(val),
        None => Err(ErrorKind::ConstEvaluationFailed(tag).into()),
    }
}

fn generate_expressions(symbol_table: &SymbolTable, input: &[ast::Expression]) -> error::Result<Vec<ir::Expr>> {
    let mut result = Vec::new();
    for expression in input {
        result.push(generate_expression(symbol_table, expression)?);
    }
    Ok(result)
}

fn generate_expression(symbol_table: &SymbolTable, input: &ast::Expression) -> error::Result<ir::Expr> {
    match *input {
        ast::Expression::ArrayIndex(ref data) => if let Some(symbol_ref) = symbol_table.find_symbol(&data.array) {
            Ok(ir::Expr::ArrayIndex(ir::ArrayIndexData::new(
                data.tag,
                symbol_ref,
                Box::new(generate_expression(symbol_table, &data.index)?),
                Type::Unresolved,
            )))
        } else {
            Err(ErrorKind::SymbolNotFound(data.tag, SymbolName::clone(&data.array)).into())
        },
        ast::Expression::BinaryOp(ref data) => Ok(ir::Expr::BinaryOp(ir::BinaryOpData::new(
            data.tag,
            data.op,
            Box::new(generate_expression(symbol_table, &data.left)?),
            Box::new(generate_expression(symbol_table, &data.right)?),
        ))),
        ast::Expression::Name(ref data) => if let Some(symbol_ref) = symbol_table.find_symbol(&data.name) {
            Ok(ir::Expr::Symbol(
                ir::SymbolData::new(data.tag, symbol_ref, Type::Unresolved),
            ))
        } else {
            Err(ErrorKind::SymbolNotFound(data.tag, SymbolName::clone(&data.name)).into())
        },
        ast::Expression::Number(ref data) => Ok(ir::Expr::Number(ir::NumberData::new(
            data.tag,
            TypedValue::UnresolvedInt(data.value),
        ))),
        ast::Expression::CallFunction(ref data) => {
            let function = SymbolName::clone(&data.name);
            let args = generate_expressions(symbol_table, &data.arguments)?;
            Ok(ir::Expr::Call(ir::CallData::new(
                data.tag,
                function,
                args,
                Type::Unresolved,
            )))
        }
        _ => panic!("not an expression: {:?}", input),
    }
}
