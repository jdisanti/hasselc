use ast;
use llir;
use ir;

const DATA_STACK_POINTER_LOCATION: llir::Location = llir::Location::Global(0x0000);
const RETURN_LOCATION_LO: llir::Location = llir::Location::Global(0x0001);
const RETURN_LOCATION_HI: llir::Location = llir::Location::Global(0x0002);

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
                block.statements = generate_body(&mut *symbol_table.borrow_mut(), None, body)?;
                block.frame_size = calculate_frame_size(&*symbol_table.borrow());
                blocks.push(block);
            },
            ir::IR::FunctionBlock { ref location, ref local_symbols, ref body, ref metadata, .. } => {
                let name = metadata.borrow().name.clone();
                let mut block = llir::Block::new(Some(name.clone()), match *location {
                    Some(ir::Location::Global(val)) => llir::Location::Global(val),
                    None => llir::Location::UnresolvedGlobal(name.clone()),
                    _ => unreachable!()
                });
                block.statements = generate_body(&mut *local_symbols.borrow_mut(), Some(&name), body)?;
                block.frame_size = calculate_frame_size(&*local_symbols.borrow());
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

fn generate_body(symbol_table: &mut ir::SymbolTable, frame: Option<&String>, input: &Vec<ir::Statement>) -> Result<Vec<llir::Statement>, ()> {
    let mut statements = Vec::new();
    for irstmt in input {
        match *irstmt {
            ir::Statement::Call(ref expr) => {
                drop(resolve_expr_to_value(&mut statements, frame, symbol_table, expr)?);
            },
            ir::Statement::Assign { ref symbol, ref value } => {
                if let Some((ref _typ, ref location)) = symbol_table.variable(symbol) {
                    let resolved_value = resolve_expr_to_value(&mut statements, frame, symbol_table, value)?;
                    statements.push(llir::Statement::Store {
                        dest: convert_location(frame, location)?,
                        value: resolved_value
                    });
                } else {
                    // TODO: error
                    unimplemented!()
                }
            },
            ir::Statement::Return(ref expr) => {
                let value = resolve_expr_to_value(&mut statements, frame, symbol_table, expr)?;
                // TODO: 16-bit values
                statements.push(llir::Statement::Store {
                    dest: RETURN_LOCATION_LO,
                    value: value
                });
                statements.push(llir::Statement::Return);
            }
            _ => { /* TODO */ }
        }
    }
    Ok(statements)
}

fn resolve_expr_to_value(statements: &mut Vec<llir::Statement>, frame: Option<&String>, symbol_table: &mut ir::SymbolTable,
        expr: &ir::Expr) -> Result<llir::Value, ()> {
    match *expr {
        ir::Expr::Number(num) => Ok(llir::Value::Immediate(num as u8)),
        ir::Expr::Symbol(ref sref) => {
            if let Some((_, ref sym_loc)) = symbol_table.variable(sref) {
                Ok(llir::Value::Memory(convert_location(frame, sym_loc)?))
            } else {
                // TODO: error: could not resolve variable
                unimplemented!()
            }
        },
        ir::Expr::BinaryOp { ref op, ref left, ref right } => {
            let dest = convert_location(frame, &symbol_table.create_temporary_location(ast::Type::U8))?;
            let left_value = resolve_expr_to_value(statements, frame, symbol_table, &**left)?;
            let right_value = resolve_expr_to_value(statements, frame, symbol_table, &**right)?;
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
                let metadata = function.borrow();
                if metadata.parameters.len() != arguments.len() {
                    // TODO: error
                    unimplemented!()
                }

                // Push arguments to the stack
                if metadata.parameters.len() > 0 {
                    statements.push(llir::Statement::AddToDataStackPointer(llir::SPOffset::FrameSize(metadata.name.clone())));

                    let mut frame_offset = 0;
                    for (i, argument) in arguments.iter().enumerate() {
                        let value = resolve_expr_to_value(statements, frame, symbol_table, argument)?;
                        statements.push(llir::Statement::Store {
                            dest: llir::Location::FrameOffset(metadata.name.clone(), frame_offset),
                            value: value
                        });
                        let name_type = &metadata.parameters[i];
                        frame_offset += name_type.type_name.size() as i8;
                    }
                }

                // Jump to the routine
                statements.push(llir::Statement::JumpRoutine {
                    location: llir::Location::UnresolvedGlobal(symbol.0.clone())
                });

                // Restore the stack pointer
                statements.push(llir::Statement::AddToDataStackPointer(llir::SPOffset::NegativeFrameSize(metadata.name.clone())));

                Ok(llir::Value::Memory(RETURN_LOCATION_LO))
            } else {
                // TODO: error
                unimplemented!()
            }
        },
    }
}

fn convert_location(frame: Option<&String>, input: &ir::Location) -> Result<llir::Location, ()> {
    let location = match *input {
        ir::Location::UndeterminedGlobal => unreachable!(),
        ir::Location::Global(addr) => llir::Location::Global(addr),
        ir::Location::FrameOffset(offset) => match frame {
            Some(name) => llir::Location::FrameOffset(name.clone(), offset),
            None => llir::Location::DataStackOffset(offset),
        }
    };
    Ok(location)
}