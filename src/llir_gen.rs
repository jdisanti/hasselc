use std::sync::Arc;
use ast;
use llir;
use ir;
use error;
use symbol_table::{Location, SymbolRef, SymbolTable};

const RETURN_LOCATION_LO: llir::Location = llir::Location::Global(0x0001);

pub fn generate_llir(input: &[ir::Block]) -> error::Result<Vec<llir::FrameBlock>> {
    let mut blocks = Vec::new();
    for irblock in input {
        // TODO: If we are an anonymous block, should we add code to prep the
        // stack pointer so that the required frame size is actually available?
        let name = SymbolRef::clone(&irblock.metadata.read().unwrap().name);
        let mut block = llir::FrameBlock::new(
            Some(SymbolRef::clone(&name)),
            match irblock.location {
                Some(Location::Global(val)) => llir::Location::Global(val),
                None => llir::Location::UnresolvedGlobal(SymbolRef::clone(&name)),
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
) -> error::Result<Vec<llir::RunBlock>> {
    let mut blocks = Vec::new();
    let mut current_block = llir::RunBlock::new(symbol_table.new_block_name());

    for irstmt in input {
        match *irstmt {
            ir::Statement::Assign(ref data) => if let Some(ref variable) = symbol_table.variable(&data.symbol) {
                let resolved_value = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame,
                    symbol_table,
                    &data.value,
                )?;
                current_block
                    .statements
                    .push(llir::Statement::Copy(llir::CopyData::new(
                        convert_location(frame.clone(), &variable.location),
                        resolved_value,
                    )));
            } else {
                unreachable!("variable existence should already be checked in ir_gen");
            },
            ir::Statement::Call(ref data) => {
                resolve_expr_to_value(
                    &mut current_block.statements,
                    frame,
                    symbol_table,
                    &data.call_expression,
                )?;
            }
            ir::Statement::Conditional(ref data) => {
                let mut true_blocks = generate_runs(symbol_table, frame, &data.when_true)?;
                let false_blocks = generate_runs(symbol_table, frame, &data.when_false)?;

                let after_both_block = llir::RunBlock::new(symbol_table.new_block_name());
                let last_true_block_index = true_blocks.len() - 1;
                true_blocks[last_true_block_index]
                    .statements
                    .push(llir::Statement::GoTo(Arc::clone(&after_both_block.name)));

                let condition = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame,
                    symbol_table,
                    &data.condition,
                )?;
                let destination = SymbolRef::clone(&false_blocks[0].name);
                current_block.statements.push(llir::Statement::BranchIfZero(
                    llir::BranchIfZeroData::new(condition, destination),
                ));

                blocks.push(current_block);
                blocks.extend(true_blocks);
                blocks.extend(false_blocks);
                blocks.push(after_both_block);
                current_block = llir::RunBlock::new(symbol_table.new_block_name());
            }
            ir::Statement::WhileLoop(ref data) => {
                let mut condition_block = llir::RunBlock::new(symbol_table.new_block_name());
                let mut body_blocks = generate_runs(symbol_table, frame, &data.body)?;
                let after_body_block = llir::RunBlock::new(symbol_table.new_block_name());

                let condition = resolve_expr_to_value(
                    &mut condition_block.statements,
                    frame,
                    symbol_table,
                    &data.condition,
                )?;

                condition_block
                    .statements
                    .push(llir::Statement::BranchIfZero(llir::BranchIfZeroData::new(
                        condition,
                        Arc::clone(&after_body_block.name),
                    )));

                let last_body_block_index = body_blocks.len() - 1;
                body_blocks[last_body_block_index]
                    .statements
                    .push(llir::Statement::GoTo(Arc::clone(&condition_block.name)));

                blocks.push(current_block);
                blocks.push(condition_block);
                blocks.extend(body_blocks);
                blocks.push(after_body_block);
                current_block = llir::RunBlock::new(symbol_table.new_block_name());
            }
            ir::Statement::Return(ref data) => {
                if let Some(ref expr) = data.value {
                    let value = resolve_expr_to_value(&mut current_block.statements, frame, symbol_table, expr)?;
                    // TODO: 16-bit values
                    current_block.statements.push(llir::Statement::Copy(
                        llir::CopyData::new(RETURN_LOCATION_LO, value),
                    ));
                }
                current_block.statements.push(llir::Statement::Return);
            }
            ir::Statement::GoTo(ref data) => {
                current_block
                    .statements
                    .push(llir::Statement::GoTo(Arc::clone(&data.destination)));
            }
            _ => unimplemented!("llir_gen: generate_runs statement"),
        }
    }

    blocks.push(current_block);
    Ok(blocks)
}

fn resolve_expr_to_value(
    statements: &mut Vec<llir::Statement>,
    frame: &Option<Arc<String>>,
    symbol_table: &mut SymbolTable,
    expr: &ir::Expr,
) -> error::Result<llir::Value> {
    match *expr {
        ir::Expr::Number(num) => Ok(llir::Value::Immediate(num as u8)),
        ir::Expr::Symbol(ref sref) => {
            if let Some(ref variable) = symbol_table.variable(sref) {
                Ok(llir::Value::Memory(
                    convert_location(frame.clone(), &variable.location),
                ))
            } else {
                // TODO: error: could not resolve variable
                unimplemented!()
            }
        }
        ir::Expr::BinaryOp {
            ref op,
            ref left,
            ref right,
        } => {
            let dest = convert_location(
                frame.clone(),
                &symbol_table.create_temporary_location(ast::Type::U8),
            );
            let left_value = resolve_expr_to_value(statements, frame, symbol_table, &**left)?;
            let right_value = resolve_expr_to_value(statements, frame, symbol_table, &**right)?;
            let bin_op_data = llir::BinaryOpData::new(dest.clone(), left_value.clone(), right_value.clone());
            let bin_op_inverted_data = llir::BinaryOpData::new(dest.clone(), right_value, left_value);
            match *op {
                ast::BinaryOperator::Add => statements.push(llir::Statement::Add(bin_op_data)),
                ast::BinaryOperator::Sub => statements.push(llir::Statement::Subtract(bin_op_data)),
                ast::BinaryOperator::Equal => statements.push(llir::Statement::CompareEq(bin_op_data)),
                ast::BinaryOperator::NotEqual => statements.push(llir::Statement::CompareNotEq(bin_op_data)),
                ast::BinaryOperator::LessThan => statements.push(llir::Statement::CompareLt(bin_op_data)),
                ast::BinaryOperator::LessThanEqual => {
                    statements.push(llir::Statement::CompareGte(bin_op_inverted_data))
                }
                ast::BinaryOperator::GreaterThan => statements.push(llir::Statement::CompareLt(bin_op_inverted_data)),
                ast::BinaryOperator::GreaterThanEqual => statements.push(llir::Statement::CompareGte(bin_op_data)),
                _ => unimplemented!(),
            };
            Ok(llir::Value::Memory(dest))
        }
        ir::Expr::Call {
            ref symbol,
            ref arguments,
        } => {
            if let Some(function) = symbol_table.function(symbol) {
                let metadata = function.read().unwrap();
                if metadata.parameters.len() != arguments.len() {
                    // TODO: error
                    unimplemented!()
                }

                // Push arguments to the stack
                let mut argument_values = Vec::new();
                for argument in arguments {
                    argument_values.push(resolve_expr_to_value(
                        statements,
                        frame,
                        symbol_table,
                        argument,
                    )?)
                }
                statements.push(llir::Statement::AddToDataStackPointer(
                    llir::SPOffset::FrameSize(SymbolRef::clone(&metadata.name)),
                ));
                if !metadata.parameters.is_empty() {
                    let mut frame_offset = 0;
                    for (i, argument_value) in argument_values.into_iter().enumerate() {
                        statements.push(llir::Statement::Copy(llir::CopyData::new(
                            llir::Location::FrameOffset(SymbolRef::clone(&metadata.name), frame_offset),
                            offset_call(SymbolRef::clone(&metadata.name), argument_value),
                        )));
                        let name_type = &metadata.parameters[i];
                        frame_offset += name_type.type_name.size() as i8;
                    }
                }

                // Jump to the routine
                statements.push(llir::Statement::JumpRoutine(
                    llir::Location::UnresolvedGlobal(SymbolRef::clone(symbol)),
                ));

                // Restore the stack pointer
                statements.push(llir::Statement::AddToDataStackPointer(
                    llir::SPOffset::NegativeFrameSize(SymbolRef::clone(&metadata.name)),
                ));

                let dest = convert_location(
                    frame.clone(),
                    &symbol_table.create_temporary_location(ast::Type::U8),
                );
                statements.push(llir::Statement::Copy(llir::CopyData::new(
                    dest.clone(),
                    llir::Value::Memory(RETURN_LOCATION_LO),
                )));

                Ok(llir::Value::Memory(dest))
            } else {
                // TODO: error
                unimplemented!()
            }
        }
    }
}

fn offset_call(calling_frame: Arc<String>, value: llir::Value) -> llir::Value {
    match value {
        llir::Value::Memory(location) => llir::Value::Memory(match location {
            llir::Location::FrameOffset(frame, offset) => {
                llir::Location::FrameOffsetBeforeCall(frame, calling_frame, offset)
            }
            _ => location,
        }),
        _ => value,
    }
}

fn convert_location(frame: Option<Arc<String>>, input: &Location) -> llir::Location {
    match *input {
        Location::UndeterminedGlobal => unreachable!(),
        Location::Global(addr) => llir::Location::Global(addr),
        Location::FrameOffset(offset) => match frame {
            Some(name) => llir::Location::FrameOffset(Arc::clone(&name), offset),
            None => llir::Location::DataStackOffset(offset),
        },
    }
}
