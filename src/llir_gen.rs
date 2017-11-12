use std::sync::Arc;
use ast;
use llir;
use ir;
use error;

const RETURN_LOCATION_LO: llir::Location = llir::Location::Global(0x0001);
const RETURN_LOCATION_HI: llir::Location = llir::Location::Global(0x0002);

pub fn generate_llir(input: &Vec<ir::IR>) -> error::Result<Vec<llir::Block>> {
    let mut blocks = Vec::new();
    for irblock in input {
        match *irblock {
            ir::IR::AnonymousBlock {
                ref symbol_table,
                ref location,
                ref body,
            } => {
                let mut block = llir::Block::new(
                    None,
                    match *location {
                        Some(ir::Location::Global(val)) => llir::Location::Global(val),
                        None => llir::Location::UnresolvedBlock,
                        _ => unreachable!(),
                    },
                );
                block.statements = generate_body(&mut *symbol_table.write().unwrap(), None, body)?;
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
                let mut block = llir::Block::new(
                    Some(name.clone()),
                    match *location {
                        Some(ir::Location::Global(val)) => llir::Location::Global(val),
                        None => llir::Location::UnresolvedGlobal(name.clone()),
                        _ => unreachable!(),
                    },
                );
                block.statements = generate_body(&mut *local_symbols.write().unwrap(), Some(name), body)?;
                block.frame_size = calculate_frame_size(&*local_symbols.read().unwrap());
                blocks.push(block);
            }
        }
    }
    Ok(blocks)
}

fn calculate_frame_size(symbol_table: &ir::SymbolTable) -> i8 {
    let mut size = 0;
    for &(typ, _) in symbol_table.variables.values() {
        size += typ.size() as i8;
    }
    size
}

fn generate_body(
    symbol_table: &mut ir::SymbolTable,
    frame: Option<Arc<String>>,
    input: &Vec<ir::Statement>,
) -> error::Result<Vec<llir::Statement>> {
    let mut statements = Vec::new();
    for irstmt in input {
        match *irstmt {
            ir::Statement::Call(ref expr) => {
                drop(resolve_expr_to_value(
                    &mut statements,
                    frame.clone(),
                    symbol_table,
                    expr,
                )?);
            }
            ir::Statement::Assign {
                ref symbol,
                ref value,
            } => {
                if let Some((ref _typ, ref location)) = symbol_table.variable(symbol) {
                    let resolved_value = resolve_expr_to_value(&mut statements, frame.clone(), symbol_table, value)?;
                    statements.push(llir::Statement::Store {
                        dest: convert_location(frame.clone(), location),
                        value: resolved_value,
                    });
                } else {
                    // TODO: error
                    unimplemented!()
                }
            }
            ir::Statement::Return(ref expr) => {
                let value = resolve_expr_to_value(&mut statements, frame.clone(), symbol_table, expr)?;
                // TODO: 16-bit values
                statements.push(llir::Statement::Store {
                    dest: RETURN_LOCATION_LO,
                    value: value,
                });
                statements.push(llir::Statement::Return);
            }
            _ => { /* TODO */ }
        }
    }
    Ok(statements)
}

fn resolve_expr_to_value(
    statements: &mut Vec<llir::Statement>,
    frame: Option<Arc<String>>,
    symbol_table: &mut ir::SymbolTable,
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
            match *op {
                ast::BinaryOperator::Add => {
                    statements.push(llir::Statement::Add {
                        dest: dest.clone(),
                        left: left_value,
                        right: right_value,
                    });
                }
                ast::BinaryOperator::Sub => {
                    statements.push(llir::Statement::Subtract {
                        dest: dest.clone(),
                        left: left_value,
                        right: right_value,
                    });
                }
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
                        statements.push(llir::Statement::Store {
                            dest: llir::Location::FrameOffset(metadata.name.clone(), frame_offset),
                            value: offset_call(metadata.name.clone(), argument_value),
                        });
                        let name_type = &metadata.parameters[i];
                        frame_offset += name_type.type_name.size() as i8;
                    }
                }

                // Jump to the routine
                statements.push(llir::Statement::JumpRoutine {
                    location: llir::Location::UnresolvedGlobal(symbol.0.clone()),
                });

                // Restore the stack pointer
                statements.push(llir::Statement::AddToDataStackPointer(
                    llir::SPOffset::NegativeFrameSize(metadata.name.clone()),
                ));

                Ok(llir::Value::Memory(RETURN_LOCATION_LO))
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

fn convert_location(frame: Option<Arc<String>>, input: &ir::Location) -> llir::Location {
    let location = match *input {
        ir::Location::UndeterminedGlobal => unreachable!(),
        ir::Location::Global(addr) => llir::Location::Global(addr),
        ir::Location::FrameOffset(offset) => match frame {
            Some(name) => llir::Location::FrameOffset(name.clone(), offset),
            None => llir::Location::DataStackOffset(offset),
        },
    };
    location
}
