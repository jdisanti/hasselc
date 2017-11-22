use std::sync::Arc;
use ast;
use error;
use ir;
use llir::{AddToDataStackPointerData, BinaryOpData, BranchIfZeroData, CopyData, FrameBlock, GoToData, JumpRoutineData, Location, ReturnData, RunBlock,
           SPOffset, Statement, Value};
use symbol_table::{self, SymbolRef, SymbolTable};

const RETURN_LOCATION_LO: Location = Location::Global(0x0001);

pub fn generate_llir(input: &[ir::Block]) -> error::Result<Vec<FrameBlock>> {
    let mut blocks = Vec::new();
    for irblock in input {
        // TODO: If we are an anonymous block, should we add code to prep the
        // stack pointer so that the required frame size is actually available?
        let name = SymbolRef::clone(&irblock.metadata.read().unwrap().name);
        let mut block = FrameBlock::new(
            Some(SymbolRef::clone(&name)),
            match irblock.location {
                Some(symbol_table::Location::Global(val)) => Location::Global(val),
                None => Location::UnresolvedGlobal(SymbolRef::clone(&name)),
                _ => unreachable!(),
            },
        );
        block.runs = generate_runs(
            &mut irblock.symbol_table.write().unwrap(),
            &Some(name),
            &irblock.body,
        )?;
        block.frame_size = calculate_frame_size(&irblock.symbol_table.read().unwrap());
        blocks.push(block);
    }
    Ok(blocks)
}

fn calculate_frame_size(symbol_table: &SymbolTable) -> i8 {
    let mut size = 0;
    for variable in symbol_table.variables() {
        size += variable.type_name.size() as i8;
    }
    size
}

fn generate_runs(
    symbol_table: &mut SymbolTable,
    frame: &Option<Arc<String>>,
    input: &[ir::Statement],
) -> error::Result<Vec<RunBlock>> {
    let mut blocks = Vec::new();
    let mut current_block = RunBlock::new(symbol_table.new_block_name());

    for irstmt in input {
        match *irstmt {
            ir::Statement::Assign(ref data) => if let Some(ref variable) = symbol_table.variable(&data.symbol) {
                let resolved_value = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame,
                    symbol_table,
                    &data.value,
                )?;
                current_block.statements.push(Statement::Copy(CopyData::new(
                    data.tag,
                    convert_location(frame.clone(), &variable.location),
                    resolved_value,
                )));
            } else {
                unreachable!("variable existence should already be checked in ir_gen");
            },
            ir::Statement::Call(ref data) => {
                generate_function_call(&mut current_block.statements, frame, symbol_table, data)?;
            }
            ir::Statement::Conditional(ref data) => {
                let mut true_blocks = generate_runs(symbol_table, frame, &data.when_true)?;
                let false_blocks = generate_runs(symbol_table, frame, &data.when_false)?;

                let after_both_block = RunBlock::new(symbol_table.new_block_name());
                let last_true_block_index = true_blocks.len() - 1;
                true_blocks[last_true_block_index]
                    .statements
                    .push(Statement::GoTo(GoToData::new(
                        data.tag,
                        SymbolRef::clone(&after_both_block.name),
                    )));

                let condition = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame,
                    symbol_table,
                    &data.condition,
                )?;
                let destination = SymbolRef::clone(&false_blocks[0].name);
                current_block.statements.push(Statement::BranchIfZero(
                    BranchIfZeroData::new(data.tag, condition, destination),
                ));

                blocks.push(current_block);
                blocks.extend(true_blocks);
                blocks.extend(false_blocks);
                blocks.push(after_both_block);
                current_block = RunBlock::new(symbol_table.new_block_name());
            }
            ir::Statement::WhileLoop(ref data) => {
                let mut condition_block = RunBlock::new(symbol_table.new_block_name());
                let mut body_blocks = generate_runs(symbol_table, frame, &data.body)?;
                let after_body_block = RunBlock::new(symbol_table.new_block_name());

                let condition = resolve_expr_to_value(
                    &mut condition_block.statements,
                    frame,
                    symbol_table,
                    &data.condition,
                )?;

                condition_block
                    .statements
                    .push(Statement::BranchIfZero(BranchIfZeroData::new(
                        data.tag,
                        condition,
                        Arc::clone(&after_body_block.name),
                    )));

                let last_body_block_index = body_blocks.len() - 1;
                body_blocks[last_body_block_index]
                    .statements
                    .push(Statement::GoTo(GoToData::new(
                        data.tag,
                        SymbolRef::clone(&condition_block.name),
                    )));

                blocks.push(current_block);
                blocks.push(condition_block);
                blocks.extend(body_blocks);
                blocks.push(after_body_block);
                current_block = RunBlock::new(symbol_table.new_block_name());
            }
            ir::Statement::Return(ref data) => {
                if let Some(ref expr) = data.value {
                    let value = resolve_expr_to_value(&mut current_block.statements, frame, symbol_table, expr)?;
                    // TODO: 16-bit values
                    current_block.statements.push(Statement::Copy(
                        CopyData::new(data.tag, RETURN_LOCATION_LO, value),
                    ));
                }
                current_block.statements.push(Statement::Return(ReturnData::new(data.tag)));
            }
            ir::Statement::GoTo(ref data) => {
                current_block.statements.push(Statement::GoTo(
                    GoToData::new(data.tag, Arc::clone(&data.destination)),
                ));
            }
            _ => unimplemented!("llir_gen: generate_runs statement"),
        }
    }

    blocks.push(current_block);
    Ok(blocks)
}

fn resolve_expr_to_value(
    statements: &mut Vec<Statement>,
    frame: &Option<Arc<String>>,
    symbol_table: &mut SymbolTable,
    expr: &ir::Expr,
) -> error::Result<Value> {
    match *expr {
        ir::Expr::Number(ref data) => Ok(Value::Immediate(data.value as u8)),
        ir::Expr::Symbol(ref data) => {
            if let Some(ref variable) = symbol_table.variable(&data.name) {
                Ok(Value::Memory(
                    convert_location(frame.clone(), &variable.location),
                ))
            } else {
                Err(error::ErrorKind::SymbolNotFound(data.tag, SymbolRef::clone(&data.name)).into())
            }
        }
        ir::Expr::BinaryOp(ref data) => {
            let dest = convert_location(
                frame.clone(),
                &symbol_table.create_temporary_location(ast::Type::U8),
            );
            let left_value = resolve_expr_to_value(statements, frame, symbol_table, &*data.left)?;
            let right_value = resolve_expr_to_value(statements, frame, symbol_table, &*data.right)?;
            let bin_op_data = BinaryOpData::new(
                data.tag,
                dest.clone(),
                left_value.clone(),
                right_value.clone(),
            );
            let bin_op_inverted_data = BinaryOpData::new(data.tag, dest.clone(), right_value, left_value);
            match data.op {
                ast::BinaryOperator::Add => statements.push(Statement::Add(bin_op_data)),
                ast::BinaryOperator::Sub => statements.push(Statement::Subtract(bin_op_data)),
                ast::BinaryOperator::Equal => statements.push(Statement::CompareEq(bin_op_data)),
                ast::BinaryOperator::NotEqual => statements.push(Statement::CompareNotEq(bin_op_data)),
                ast::BinaryOperator::LessThan => statements.push(Statement::CompareLt(bin_op_data)),
                ast::BinaryOperator::LessThanEqual => statements.push(Statement::CompareGte(bin_op_inverted_data)),
                ast::BinaryOperator::GreaterThan => statements.push(Statement::CompareLt(bin_op_inverted_data)),
                ast::BinaryOperator::GreaterThanEqual => statements.push(Statement::CompareGte(bin_op_data)),
                _ => unimplemented!(),
            };
            Ok(Value::Memory(dest))
        }
        ir::Expr::Call(ref data) => generate_function_call(statements, frame, symbol_table, data),
    }
}

fn generate_function_call(
    statements: &mut Vec<Statement>,
    frame: &Option<Arc<String>>,
    symbol_table: &mut SymbolTable,
    call_data: &ir::CallData,
) -> error::Result<Value> {
    if let Some(function) = symbol_table.function(&call_data.function) {
        let metadata = function.read().unwrap();
        if metadata.parameters.len() != call_data.arguments.len() {
            return Err(
                error::ErrorKind::ExpectedNArgumentsGotM(
                    call_data.tag,
                    SymbolRef::clone(&call_data.function),
                    metadata.parameters.len(),
                    call_data.arguments.len(),
                ).into(),
            );
        }

        // Push arguments to the stack
        let mut argument_values = Vec::new();
        for argument in &call_data.arguments {
            argument_values.push(resolve_expr_to_value(
                statements,
                frame,
                symbol_table,
                argument,
            )?)
        }
        statements.push(Statement::AddToDataStackPointer(AddToDataStackPointerData::new(
            call_data.tag,
            SPOffset::FrameSize(SymbolRef::clone(&metadata.name)),
        )));
        if !metadata.parameters.is_empty() {
            let mut frame_offset = 0;
            for (i, argument_value) in argument_values.into_iter().enumerate() {
                statements.push(Statement::Copy(CopyData::new(
                    call_data.tag,
                    Location::FrameOffset(SymbolRef::clone(&metadata.name), frame_offset),
                    offset_call(SymbolRef::clone(&metadata.name), argument_value),
                )));
                let name_type = &metadata.parameters[i];
                frame_offset += name_type.type_name.size() as i8;
            }
        }

        // Jump to the routine
        statements.push(Statement::JumpRoutine(JumpRoutineData::new(
            call_data.tag,
            Location::UnresolvedGlobal(SymbolRef::clone(&call_data.function)),
        )));

        // Restore the stack pointer
        statements.push(Statement::AddToDataStackPointer(AddToDataStackPointerData::new(
            call_data.tag,
            SPOffset::NegativeFrameSize(SymbolRef::clone(&metadata.name)),
        )));

        let dest = convert_location(
            frame.clone(),
            &symbol_table.create_temporary_location(ast::Type::U8),
        );
        statements.push(Statement::Copy(CopyData::new(
            call_data.tag,
            dest.clone(),
            Value::Memory(RETURN_LOCATION_LO),
        )));

        Ok(Value::Memory(dest))
    } else {
        Err(error::ErrorKind::SymbolNotFound(call_data.tag, SymbolRef::clone(&call_data.function)).into())
    }
}

fn offset_call(calling_frame: Arc<String>, value: Value) -> Value {
    match value {
        Value::Memory(location) => Value::Memory(match location {
            Location::FrameOffset(frame, offset) => Location::FrameOffsetBeforeCall(frame, calling_frame, offset),
            _ => location,
        }),
        _ => value,
    }
}

fn convert_location(frame: Option<Arc<String>>, input: &symbol_table::Location) -> Location {
    match *input {
        symbol_table::Location::UndeterminedGlobal => unreachable!(),
        symbol_table::Location::Global(addr) => Location::Global(addr),
        symbol_table::Location::FrameOffset(offset) => match frame {
            Some(name) => Location::FrameOffset(Arc::clone(&name), offset),
            None => Location::DataStackOffset(offset),
        },
    }
}
