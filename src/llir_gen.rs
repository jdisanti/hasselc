use std::sync::Arc;
use ast;
use llir;
use ir;
use error;
use symbol_table::{Location, SymbolTable};

const RETURN_LOCATION_LO: llir::Location = llir::Location::Global(0x0001);
const RETURN_LOCATION_HI: llir::Location = llir::Location::Global(0x0002);

pub fn generate_llir(input: &Vec<ir::IR>) -> error::Result<Vec<llir::FrameBlock>> {
    let mut blocks = Vec::new();
    for irblock in input {
        match *irblock {
            ir::IR::AnonymousBlock {
                ref symbol_table,
                ref location,
                ref body,
            } => {
                let mut block = llir::FrameBlock::new(
                    None,
                    match *location {
                        Some(Location::Global(val)) => llir::Location::Global(val),
                        None => llir::Location::UnresolvedBlock,
                        _ => unreachable!(),
                    },
                );
                // TODO: Add code to prep the stack pointer so that the required frame size is actually available?
                block.runs = generate_runs(&mut *symbol_table.write().unwrap(), None, body)?;
                block.frame_size = calculate_frame_size(&*symbol_table.read().unwrap());
                blocks.push(block);
            }
            ir::IR::FunctionBlock {
                ref location,
                ref local_symbols,
                ref body,
                ref metadata,
                ..
            } => {
                let name = metadata.read().unwrap().name.clone();
                let mut block = llir::FrameBlock::new(
                    Some(name.clone()),
                    match *location {
                        Some(Location::Global(val)) => llir::Location::Global(val),
                        None => llir::Location::UnresolvedGlobal(name.clone()),
                        _ => unreachable!(),
                    },
                );
                block.runs = generate_runs(&mut *local_symbols.write().unwrap(), Some(name), body)?;
                block.frame_size = calculate_frame_size(&*local_symbols.read().unwrap());
                blocks.push(block);
            }
        }
    }
    Ok(blocks)
}

fn calculate_frame_size(symbol_table: &SymbolTable) -> i8 {
    let mut size = 0;
    for &(typ, _) in symbol_table.variables.values() {
        size += typ.size() as i8;
    }
    size
}

fn generate_runs(
    symbol_table: &mut SymbolTable,
    frame: Option<Arc<String>>,
    input: &Vec<ir::Statement>,
) -> error::Result<Vec<llir::RunBlock>> {
    let mut blocks = Vec::new();
    let mut current_block = llir::RunBlock::new(symbol_table.new_run_block_name());

    for irstmt in input {
        match *irstmt {
            ir::Statement::Assign {
                ref symbol,
                ref value,
            } => {
                if let Some((ref _typ, ref location)) = symbol_table.variable(symbol) {
                    let resolved_value = resolve_expr_to_value(
                        &mut current_block.statements,
                        frame.clone(),
                        symbol_table,
                        value,
                    )?;
                    current_block
                        .statements
                        .push(llir::Statement::Copy(llir::CopyData::new(
                            convert_location(frame.clone(), location),
                            resolved_value,
                        )));
                } else {
                    // TODO: error
                    unimplemented!()
                }
            }
            ir::Statement::Call(ref expr) => {
                drop(resolve_expr_to_value(
                    &mut current_block.statements,
                    frame.clone(),
                    symbol_table,
                    expr,
                )?);
            }
            ir::Statement::Conditional(ref data) => {
                let mut true_blocks = generate_runs(symbol_table, frame.clone(), &data.when_true)?;
                let false_blocks = generate_runs(symbol_table, frame.clone(), &data.when_false)?;

                let after_both_block = llir::RunBlock::new(symbol_table.new_run_block_name());
                let last_true_block_index = true_blocks.len() - 1;
                true_blocks[last_true_block_index]
                    .statements
                    .push(llir::Statement::GoTo(Arc::clone(&after_both_block.name)));

                let condition = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame.clone(),
                    symbol_table,
                    &data.condition,
                )?;
                let destination = false_blocks[0].name.clone();
                current_block.statements.push(llir::Statement::BranchIfZero(
                    llir::BranchIfZeroData::new(condition, destination),
                ));

                blocks.push(current_block);
                blocks.extend(true_blocks);
                blocks.extend(false_blocks);
                blocks.push(after_both_block);
                current_block = llir::RunBlock::new(symbol_table.new_run_block_name());
            }
            ir::Statement::WhileLoop(ref data) => {
                let mut condition_block = llir::RunBlock::new(symbol_table.new_run_block_name());
                let mut body_blocks = generate_runs(symbol_table, frame.clone(), &data.body)?;
                let after_body_block = llir::RunBlock::new(symbol_table.new_run_block_name());

                let condition = resolve_expr_to_value(
                    &mut condition_block.statements,
                    frame.clone(),
                    symbol_table,
                    &data.condition,
                )?;

                condition_block.statements.push(llir::Statement::BranchIfZero(
                    llir::BranchIfZeroData::new(condition, Arc::clone(&after_body_block.name)),
                ));

                let last_body_block_index = body_blocks.len() - 1;
                body_blocks[last_body_block_index].statements.push(llir::Statement::GoTo(Arc::clone(&condition_block.name)));

                blocks.push(current_block);
                blocks.push(condition_block);
                blocks.extend(body_blocks);
                blocks.push(after_body_block);
                current_block = llir::RunBlock::new(symbol_table.new_run_block_name());
            }
            ir::Statement::Return(ref optional_expr) => {
                if let Some(ref expr) = *optional_expr {
                    let value = resolve_expr_to_value(
                        &mut current_block.statements,
                        frame.clone(),
                        symbol_table,
                        expr,
                    )?;
                    // TODO: 16-bit values
                    current_block.statements.push(llir::Statement::Copy(
                        llir::CopyData::new(RETURN_LOCATION_LO, value),
                    ));
                }
                current_block.statements.push(llir::Statement::Return);
            }
            ir::Statement::GoTo(ref name) => {
                current_block
                    .statements
                    .push(llir::Statement::GoTo(name.clone()));
            }
            _ => { /* TODO */ }
        }
    }

    blocks.push(current_block);
    Ok(blocks)
}

fn resolve_expr_to_value(
    statements: &mut Vec<llir::Statement>,
    frame: Option<Arc<String>>,
    symbol_table: &mut SymbolTable,
    expr: &ir::Expr,
) -> error::Result<llir::Value> {
    match *expr {
        ir::Expr::Number(num) => Ok(llir::Value::Immediate(num as u8)),
        ir::Expr::Symbol(ref sref) => {
            if let Some((_, ref sym_loc)) = symbol_table.variable(sref) {
                Ok(llir::Value::Memory(
                    convert_location(frame.clone(), sym_loc),
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
            let left_value = resolve_expr_to_value(statements, frame.clone(), symbol_table, &**left)?;
            let right_value = resolve_expr_to_value(statements, frame.clone(), symbol_table, &**right)?;
            let bin_op_data = llir::BinaryOpData::new(dest.clone(), left_value.clone(), right_value.clone());
            let bin_op_inverted_data = llir::BinaryOpData::new(dest.clone(), right_value, left_value);
            match *op {
                ast::BinaryOperator::Add => statements.push(llir::Statement::Add(bin_op_data)),
                ast::BinaryOperator::Sub => statements.push(llir::Statement::Subtract(bin_op_data)),
                ast::BinaryOperator::Equal => statements.push(llir::Statement::CompareEq(bin_op_data)),
                ast::BinaryOperator::NotEqual => statements.push(llir::Statement::CompareNotEq(bin_op_data)),
                ast::BinaryOperator::LessThan => statements.push(llir::Statement::CompareLt(bin_op_data)),
                ast::BinaryOperator::LessThanEqual => statements.push(llir::Statement::CompareGte(bin_op_inverted_data)),
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
                        frame.clone(),
                        symbol_table,
                        argument,
                    )?)
                }
                statements.push(llir::Statement::AddToDataStackPointer(
                    llir::SPOffset::FrameSize(metadata.name.clone()),
                ));
                if metadata.parameters.len() > 0 {
                    let mut frame_offset = 0;
                    for (i, argument_value) in argument_values.into_iter().enumerate() {
                        statements.push(llir::Statement::Copy(llir::CopyData::new(
                            llir::Location::FrameOffset(metadata.name.clone(), frame_offset),
                            offset_call(metadata.name.clone(), argument_value),
                        )));
                        let name_type = &metadata.parameters[i];
                        frame_offset += name_type.type_name.size() as i8;
                    }
                }

                // Jump to the routine
                statements.push(llir::Statement::JumpRoutine(
                    llir::Location::UnresolvedGlobal(symbol.0.clone()),
                ));

                // Restore the stack pointer
                statements.push(llir::Statement::AddToDataStackPointer(
                    llir::SPOffset::NegativeFrameSize(metadata.name.clone()),
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
    let location = match *input {
        Location::UndeterminedGlobal => unreachable!(),
        Location::Global(addr) => llir::Location::Global(addr),
        Location::FrameOffset(offset) => match frame {
            Some(name) => llir::Location::FrameOffset(name.clone(), offset),
            None => llir::Location::DataStackOffset(offset),
        },
    };
    location
}
