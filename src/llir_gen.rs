use ast;
use llir;
use ir;

const DATA_STACK_POINTER_LOCATION: llir::Location = llir::Location::Global(0x0000);
const FRAME_POINTER_LOCATION: llir::Location = llir::Location::Global(0x0001);

pub fn generate_llir(input: &Vec<ir::IR>) -> Result<Vec<llir::Block>, ()> {
    let mut blocks = Vec::new();
    for irblock in input {
        match *irblock {
            ir::IR::AnonymousBlock { ref symbol_table, ref location, ref body } => {
                let mut block = llir::Block::new(None, match *location {
                    Some(ir::Location::Global(val)) => llir::Location::Global(val),
                    None => llir::Location::UnresolvedBlock,
                    _ => unreachable!()
                });
                block.statements = generate_body(&mut *symbol_table.borrow_mut(), body)?;
                blocks.push(block);
            },
            ir::IR::FunctionBlock { ref location, ref name, ref local_symbols, ref body, .. } => {
                let mut block = llir::Block::new(Some(name.clone()), match *location {
                    Some(ir::Location::Global(val)) => llir::Location::Global(val),
                    None => llir::Location::UnresolvedGlobal(name.clone()),
                    _ => unreachable!()
                });
                block.statements = generate_body(&mut *local_symbols.borrow_mut(), body)?;
                blocks.push(block);
            }
        }
    }
    Ok(blocks)
}

fn generate_body(symbol_table: &mut ir::SymbolTable, input: &Vec<ir::Statement>) -> Result<Vec<llir::Statement>, ()> {
    let mut statements = Vec::new();
    for irstmt in input {
        match *irstmt {
            ir::Statement::Call(ref expr) => {
                drop(resolve_expr_to_value(&mut statements, symbol_table, expr)?);
            },
            ir::Statement::Assign { ref symbol, ref value } => {
                if let Some((ref _typ, ref location)) = symbol_table.variable(symbol) {
                    let resolved_value = resolve_expr_to_value(&mut statements, symbol_table, value)?;
                    statements.push(llir::Statement::Store {
                        dest: convert_location(location)?,
                        value: resolved_value
                    });
                } else {
                    // TODO: error
                    unimplemented!()
                }
            },
            ir::Statement::Return(ref expr) => {
                let value = resolve_expr_to_value(&mut statements, symbol_table, expr)?;
                statements.push(llir::Statement::Store {
                    dest: llir::Location::DataStackOffset(0),
                    value: value
                });
                statements.push(llir::Statement::Return);
            }
            _ => { /* TODO */ }
        }
    }
    Ok(statements)
}

fn resolve_expr_to_value(statements: &mut Vec<llir::Statement>, symbol_table: &mut ir::SymbolTable,
        expr: &ir::Expr) -> Result<llir::Value, ()> {
    match *expr {
        ir::Expr::Number(num) => Ok(llir::Value::Immediate(num as u8)),
        ir::Expr::Symbol(ref sref) => {
            if let Some((_, ref sym_loc)) = symbol_table.variable(sref) {
                Ok(llir::Value::Memory(convert_location(sym_loc)?))
            } else {
                // TODO: error: could not resolve variable
                unimplemented!()
            }
        },
        ir::Expr::BinaryOp { ref op, ref left, ref right } => {
            let dest = convert_location(&symbol_table.create_temporary_location(ast::Type::U8))?;
            let left_value = resolve_expr_to_value(statements, symbol_table, &**left)?;
            let right_value = resolve_expr_to_value(statements, symbol_table, &**right)?;
            match * op {
                ast::BinaryOperator::Add => {
                    statements.push(llir::Statement::Add { dest: dest.clone(), left: left_value, right: right_value });
                },
                ast::BinaryOperator::Sub => {
                    statements.push(llir::Statement::Subtract { dest: dest.clone(), left: left_value, right: right_value });
                },
                _ => unimplemented!()
            };
            Ok(llir::Value::Memory(dest))
        },
        ir::Expr::Call { ref symbol, ref arguments } => {
            if let Some(function) = symbol_table.function(symbol) {
                if function.parameters.len() != arguments.len() {
                    // TODO: error
                    unimplemented!()
                }

                // Create a temporary location for the return value
                let return_value_dest = convert_location(&symbol_table.create_temporary_location(function.return_type))?;

                // Calculate the needed stack size
                let mut stack_size: i8 = 0;
                for parameter in &function.parameters {
                    stack_size += parameter.type_name.size() as i8;
                }

                // Save the data stack pointer to the frame pointer
                statements.push(llir::Statement::AddToDataStackPointer(stack_size + 1));
                statements.push(llir::Statement::Store {
                    dest: llir::Location::DataStackOffset(-(stack_size + 1)),
                    value: llir::Value::Memory(FRAME_POINTER_LOCATION),
                });
                statements.push(llir::Statement::Store {
                    dest: FRAME_POINTER_LOCATION,
                    value: llir::Value::Memory(DATA_STACK_POINTER_LOCATION),
                });

                // Push arguments to the stack
                if stack_size != 0 {
                    let mut stack_offset = -stack_size;
                    for (i, argument) in arguments.iter().enumerate() {
                        let value = resolve_expr_to_value(statements, symbol_table, argument)?;
                        statements.push(llir::Statement::Store {
                            dest: llir::Location::DataStackOffset(stack_offset),
                            value: value
                        });
                        let name_type = &function.parameters[i];
                        stack_offset += name_type.type_name.size() as i8;
                    }
                }

                // Jump to the routine
                statements.push(llir::Statement::JumpRoutine {
                    location: llir::Location::UnresolvedGlobal(symbol.0.clone())
                });

                // Restore the frame pointer
                statements.push(llir::Statement::Store {
                    dest: FRAME_POINTER_LOCATION,
                    value: llir::Value::Memory(llir::Location::DataStackOffset(-(stack_size + 1))),
                });

                // First stack entry is the return value
                if function.return_type.size() > 0 {
                    statements.push(llir::Statement::Store {
                        dest: return_value_dest.clone(),
                        value: llir::Value::Memory(llir::Location::DataStackOffset(-stack_size)),
                    });
                }

                // Restore the stack pointer
                statements.push(llir::Statement::AddToDataStackPointer(-(stack_size + 1)));

                Ok(llir::Value::Memory(return_value_dest))
            } else {
                // TODO: error
                unimplemented!()
            }
        },
    }
}

fn convert_location(input: &ir::Location) -> Result<llir::Location, ()> {
    let location = match *input {
        ir::Location::UndeterminedGlobal => unreachable!(),
        ir::Location::Global(addr) => llir::Location::Global(addr),
        ir::Location::StackOffset(offset) => llir::Location::FrameOffset(offset as i8),
    };
    Ok(location)
}