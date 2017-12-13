use std::sync::Arc;
use error;
use ir;
use llir::{binop, AddToDataStackPointerData, BinaryOpData, BranchIfZeroData, CarryMode, CopyData, FrameBlock,
           GoToData, ImmediateValue, InlineAsmData, JumpRoutineData, Location, MemoryData, ReturnData, RunBlock,
           SPOffset, Statement, Value};
use parse::ast;
use symbol_table::{self, SymbolName, SymbolRef, SymbolTable};
use src_tag::{SrcTag, SrcTagged};
use base_type::BaseType;

const RETURN_LOCATION_LO: Location = Location::Global(0x0001);

pub fn generate_llir(input: &[ir::Block]) -> error::Result<Vec<FrameBlock>> {
    let mut blocks = Vec::new();
    for irblock in input {
        // TODO: If we are an anonymous block, should we add code to prep the
        // stack pointer so that the required frame size is actually available?
        let name = SymbolName::clone(&irblock.metadata.read().unwrap().name);
        let mut block = FrameBlock::new(
            SymbolName::clone(&name),
            irblock.symbol,
            match irblock.location {
                Some(symbol_table::Location::Global(val)) => Location::Global(val),
                None => Location::UnresolvedGlobal(irblock.symbol),
                _ => unreachable!(),
            },
        );
        block.runs = generate_runs(
            &mut *irblock.symbol_table.write().unwrap(),
            irblock.symbol,
            &irblock.body,
        )?;
        block.frame_size = calculate_frame_size(&*irblock.symbol_table.read().unwrap());
        blocks.push(block);
    }
    Ok(blocks)
}

fn calculate_frame_size(symbol_table: &SymbolTable) -> i8 {
    let mut size = 0;
    for variable in symbol_table.variables() {
        size += variable.base_type.size().unwrap() as i8;
    }
    size
}

fn generate_runs(
    symbol_table: &mut SymbolTable,
    frame_ref: SymbolRef,
    input: &[ir::Statement],
) -> error::Result<Vec<RunBlock>> {
    let mut blocks = Vec::new();
    let (block_name, block_ref) = symbol_table.new_block_name();
    let mut current_block = RunBlock::new(block_name, block_ref);

    for irstmt in input {
        match *irstmt {
            ir::Statement::Assign(ref data) => {
                let right_value = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame_ref,
                    symbol_table,
                    &data.right_value,
                )?;

                let left_location = resolve_expr_to_location(
                    &mut current_block.statements,
                    frame_ref,
                    symbol_table,
                    &data.left_value,
                )?;

                generate_copy(
                    &mut current_block.statements,
                    data.tag,
                    data.value_type.as_ref().unwrap(),
                    right_value,
                    left_location,
                )?;
            }
            ir::Statement::Call(ref data) => {
                generate_function_call(&mut current_block.statements, frame_ref, symbol_table, data)?;
            }
            ir::Statement::Conditional(ref data) => {
                let mut true_blocks = generate_runs(symbol_table, frame_ref, &data.when_true)?;
                let false_blocks = generate_runs(symbol_table, frame_ref, &data.when_false)?;

                let after_both_block = RunBlock::new_tup(symbol_table.new_block_name());
                let last_true_block_index = true_blocks.len() - 1;
                true_blocks[last_true_block_index].statements.push(Statement::GoTo(
                    GoToData::new(data.tag, after_both_block.symbol),
                ));

                let condition = resolve_expr_to_value(
                    &mut current_block.statements,
                    frame_ref,
                    symbol_table,
                    &data.condition,
                )?;
                current_block.statements.push(Statement::BranchIfZero(BranchIfZeroData::new(
                    data.tag,
                    condition,
                    false_blocks[0].symbol,
                )));

                blocks.push(current_block);
                blocks.extend(true_blocks);
                blocks.extend(false_blocks);
                blocks.push(after_both_block);
                current_block = RunBlock::new_tup(symbol_table.new_block_name());
            }
            ir::Statement::InlineAsm(ref data) => {
                current_block.statements.push(Statement::InlineAsm(
                    InlineAsmData::new(data.tag, Arc::clone(&data.asm)),
                ));
            }
            ir::Statement::WhileLoop(ref data) => {
                let mut condition_block = RunBlock::new_tup(symbol_table.new_block_name());
                let mut body_blocks = generate_runs(symbol_table, frame_ref, &data.body)?;
                let after_body_block = RunBlock::new_tup(symbol_table.new_block_name());

                let condition = resolve_expr_to_value(
                    &mut condition_block.statements,
                    frame_ref,
                    symbol_table,
                    &data.condition,
                )?;

                condition_block.statements.push(Statement::BranchIfZero(BranchIfZeroData::new(
                    data.tag,
                    condition,
                    after_body_block.symbol,
                )));

                let last_body_block_index = body_blocks.len() - 1;
                body_blocks[last_body_block_index].statements.push(Statement::GoTo(
                    GoToData::new(data.tag, condition_block.symbol),
                ));

                blocks.push(current_block);
                blocks.push(condition_block);
                blocks.extend(body_blocks);
                blocks.push(after_body_block);
                current_block = RunBlock::new_tup(symbol_table.new_block_name());
            }
            ir::Statement::Return(ref data) => {
                if let Some(ref expr) = data.value {
                    let value = resolve_expr_to_value(&mut current_block.statements, frame_ref, symbol_table, expr)?;
                    generate_copy(
                        &mut current_block.statements,
                        data.tag,
                        data.value_type.as_ref().unwrap(),
                        value,
                        RETURN_LOCATION_LO,
                    )?;
                }
                current_block.statements.push(Statement::Return(ReturnData::new(data.tag)));
            }
            ir::Statement::GoTo(ref data) => {
                if let Some(symbol_ref) = symbol_table.find_symbol(&data.destination) {
                    current_block.statements.push(
                        Statement::GoTo(GoToData::new(data.tag, symbol_ref)),
                    );
                } else {
                    return Err(
                        error::ErrorKind::SymbolNotFound(data.tag, SymbolName::clone(&data.destination)).into(),
                    );
                }
            }
            _ => unimplemented!("llir_gen: generate_runs statement"),
        }
    }

    blocks.push(current_block);
    Ok(blocks)
}

fn generate_copy(
    statements: &mut Vec<Statement>,
    tag: SrcTag,
    value_type: &BaseType,
    value: Value,
    destination: Location,
) -> error::Result<()> {
    match *value_type {
        BaseType::Bool | BaseType::U8 => {
            statements.push(Statement::Copy(CopyData::new(tag, destination, value)));
        }
        BaseType::U16 |
        BaseType::Pointer(_) => {
            statements.push(Statement::Copy(CopyData::new(
                tag,
                destination.high_byte(),
                Value::high_byte(&value),
            )));
            statements.push(Statement::Copy(CopyData::new(
                tag,
                destination.low_byte(),
                Value::low_byte(&value),
            )));
        }
        BaseType::Void => unreachable!(),
    }
    Ok(())
}

fn resolve_expr_to_location(
    statements: &mut Vec<Statement>,
    frame_ref: SymbolRef,
    symbol_table: &mut SymbolTable,
    expr: &ir::Expr,
) -> error::Result<Location> {
    match resolve_expr_to_value(statements, frame_ref, symbol_table, expr)? {
        Value::Immediate(_, _) => Err(error::ErrorKind::InvalidLeftValue(expr.src_tag()).into()),
        Value::Memory(data) => Ok(data.location),
    }
}

fn resolve_expr_to_value(
    statements: &mut Vec<Statement>,
    frame_ref: SymbolRef,
    symbol_table: &mut SymbolTable,
    expr: &ir::Expr,
) -> error::Result<Value> {
    match *expr {
        ir::Expr::ArrayIndex(ref data) => {
            let array = symbol_table.variable(data.array).unwrap();
            let array_name = symbol_table.get_symbol_name(data.array).unwrap();
            let index_value = resolve_expr_to_value(statements, frame_ref, symbol_table, &data.index)?;
            match array.location {
                symbol_table::Location::UndeterminedGlobal => unreachable!(),
                symbol_table::Location::Global(addr) => Ok(Value::Memory(MemoryData::new(
                    data.array_type
                        .as_ref()
                        .unwrap()
                        .underlying_type()
                        .unwrap()
                        .clone(),
                    Location::GlobalIndexed(addr, Box::new(index_value)),
                    Some(Arc::new(format!("{}[]", array_name))),
                ))),
                symbol_table::Location::FrameOffset(_) => {
                    let addr = convert_location(
                        frame_ref,
                        &symbol_table.create_temporary_location(data.array_type.as_ref().unwrap()),
                    );
                    generate_copy(
                        statements,
                        data.tag,
                        data.array_type.as_ref().unwrap(),
                        Value::Memory(MemoryData::new(
                            data.array_type.as_ref().unwrap().clone(),
                            convert_location(frame_ref, &array.location),
                            Some(Arc::new(format!("{}[]", array_name))),
                        )),
                        addr.clone(),
                    )?;

                    // Do a full 16-bit addition to the indexed address
                    statements.push(Statement::Add(BinaryOpData::new(
                        data.tag,
                        addr.low_byte(),
                        Value::Memory(MemoryData::new(
                            BaseType::U16,
                            addr.low_byte(),
                            Some(
                                Arc::new(format!("tmp_indexed_addr_lo:{}", array_name)),
                            ),
                        )),
                        index_value,
                        CarryMode::ClearCarry,
                    )));
                    statements.push(Statement::Add(BinaryOpData::new(
                        data.tag,
                        addr.high_byte(),
                        Value::Memory(MemoryData::new(
                            BaseType::U16,
                            addr.high_byte(),
                            Some(
                                Arc::new(format!("tmp_indexed_addr_hi:{}", array_name)),
                            ),
                        )),
                        Value::Immediate(BaseType::U8, ImmediateValue::Number(0)),
                        CarryMode::DontCare,
                    )));

                    Ok(Value::Memory(MemoryData::new(
                        data.array_type
                            .as_ref()
                            .unwrap()
                            .underlying_type()
                            .unwrap()
                            .clone(),
                        match addr {
                            Location::FrameOffset(sym_ref, offset) => Location::FrameOffsetIndirect(sym_ref, offset),
                            _ => unreachable!(),
                        },
                        Some(Arc::new(format!("indexed:{}", array_name))),
                    )))
                }
            }
        }
        ir::Expr::Number(ref data) => Ok(Value::Immediate(
            data.value_type.as_ref().unwrap().clone(),
            ImmediateValue::Number(data.value),
        )),
        ir::Expr::Symbol(ref data) => {
            if let Some(ref variable) = symbol_table.variable(data.symbol) {
                match variable.base_type {
                    BaseType::Pointer(_) => {
                        match variable.location {
                            // Return the address as a U16 when naming an array without an index
                            symbol_table::Location::Global(addr) => Ok(Value::Immediate(
                                BaseType::U16,
                                ImmediateValue::Number(addr as i32),
                            )),
                            _ => Ok(Value::Memory(MemoryData::new(
                                BaseType::U16,
                                convert_location(frame_ref, &variable.location),
                                Some(symbol_table.get_symbol_name(data.symbol).unwrap()),
                            ))),
                        }
                    }
                    _ => Ok(Value::Memory(MemoryData::new(
                        variable.base_type.clone(),
                        convert_location(frame_ref, &variable.location),
                        Some(symbol_table.get_symbol_name(data.symbol).unwrap()),
                    ))),
                }
            } else if let Some(ref constant) = symbol_table.constant(data.symbol) {
                if constant.base_type.is_pointer() {
                    Ok(Value::Immediate(
                        data.value_type.as_ref().unwrap().clone(),
                        ImmediateValue::Symbol(data.symbol),
                    ))
                } else {
                    Ok(Value::Immediate(
                        data.value_type.as_ref().unwrap().clone(),
                        ImmediateValue::Number(constant.value.number()),
                    ))
                }
            } else {
                unreachable!()
            }
        }
        ir::Expr::BinaryOp(ref data) => {
            let dest_type = data.result_type.as_ref().unwrap();
            let dest = convert_location(
                frame_ref,
                &symbol_table.create_temporary_location(dest_type),
            );
            let left_value = resolve_expr_to_value(statements, frame_ref, symbol_table, &*data.left)?;
            let right_value = resolve_expr_to_value(statements, frame_ref, symbol_table, &*data.right)?;
            let bin_op_data = BinaryOpData::new(
                data.tag,
                dest.clone(),
                left_value.clone(),
                right_value.clone(),
                match data.op {
                    ast::BinaryOperator::Add => CarryMode::ClearCarry,
                    ast::BinaryOperator::Sub => CarryMode::SetCarry,
                    _ => CarryMode::DontCare,
                },
            );
            let bin_op_inverted_data = BinaryOpData::new(
                data.tag,
                dest.clone(),
                right_value.clone(),
                left_value.clone(),
                bin_op_data.carry_mode,
            );
            match data.op {
                ast::BinaryOperator::Add => {
                    binop::add::generate_add(
                        statements,
                        data.tag,
                        &dest_type,
                        &dest,
                        &left_value,
                        &right_value,
                    )?
                }
                ast::BinaryOperator::Sub => {
                    binop::add::generate_sub(
                        statements,
                        data.tag,
                        &dest_type,
                        &dest,
                        &left_value,
                        &right_value,
                    )?
                }
                // TODO: Add 16-bit and mixed 16-bit/8-bit support to remaining ops
                ast::BinaryOperator::Equal => statements.push(Statement::CompareEq(bin_op_data)),
                ast::BinaryOperator::NotEqual => statements.push(Statement::CompareNotEq(bin_op_data)),
                ast::BinaryOperator::LessThan => statements.push(Statement::CompareLt(bin_op_data)),
                ast::BinaryOperator::LessThanEqual => statements.push(Statement::CompareGte(bin_op_inverted_data)),
                ast::BinaryOperator::GreaterThan => statements.push(Statement::CompareLt(bin_op_inverted_data)),
                ast::BinaryOperator::GreaterThanEqual => statements.push(Statement::CompareGte(bin_op_data)),
                _ => unimplemented!(),
            };
            Ok(Value::Memory(MemoryData::new(BaseType::U8, dest, None)))
        }
        ir::Expr::Call(ref data) => generate_function_call(statements, frame_ref, symbol_table, data),
    }
}

fn generate_function_call(
    statements: &mut Vec<Statement>,
    frame_ref: SymbolRef,
    symbol_table: &mut SymbolTable,
    call_data: &ir::CallData,
) -> error::Result<Value> {
    if let Some(function_ref) = symbol_table.find_symbol(&call_data.function) {
        let function = symbol_table.function_by_name(&call_data.function).unwrap();
        let metadata = function.read().unwrap();
        if metadata.parameters.len() != call_data.arguments.len() {
            return Err(
                error::ErrorKind::ExpectedNArgumentsGotM(
                    call_data.tag,
                    SymbolName::clone(&call_data.function),
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
                frame_ref,
                symbol_table,
                argument,
            )?)
        }
        statements.push(Statement::AddToDataStackPointer(
            AddToDataStackPointerData::new(
                call_data.tag,
                SPOffset::FrameSize(function_ref),
            ),
        ));
        if !metadata.parameters.is_empty() {
            let mut frame_offset = 0;
            for (i, argument_value) in argument_values.into_iter().enumerate() {
                generate_copy(
                    statements,
                    call_data.tag,
                    &argument_value.value_type(),
                    offset_call(function_ref, argument_value),
                    Location::FrameOffset(function_ref, frame_offset),
                )?;
                let name_type = &metadata.parameters[i];
                frame_offset += name_type.base_type.size().unwrap() as i8;
            }
        }

        // Jump to the routine
        statements.push(Statement::JumpRoutine(JumpRoutineData::new(
            call_data.tag,
            Location::UnresolvedGlobal(function_ref),
        )));

        // Restore the stack pointer
        statements.push(Statement::AddToDataStackPointer(
            AddToDataStackPointerData::new(
                call_data.tag,
                SPOffset::NegativeFrameSize(function_ref),
            ),
        ));

        if call_data.return_type.as_ref().unwrap().size().is_some() {
            let dest = convert_location(
                frame_ref,
                &symbol_table.create_temporary_location(&BaseType::U8),
            );
            generate_copy(
                statements,
                call_data.tag,
                call_data.return_type.as_ref().unwrap(),
                Value::Memory(MemoryData::new(
                    call_data.return_type.as_ref().unwrap().clone(),
                    RETURN_LOCATION_LO,
                    None,
                )),
                dest.clone(),
            )?;

            Ok(Value::Memory(MemoryData::new(
                call_data.return_type.as_ref().unwrap().clone(),
                dest,
                None,
            )))
        } else {
            Ok(Value::Memory(MemoryData::new(
                call_data.return_type.as_ref().unwrap().clone(),
                RETURN_LOCATION_LO,
                None,
            )))
        }
    } else {
        Err(
            error::ErrorKind::SymbolNotFound(call_data.tag, SymbolName::clone(&call_data.function)).into(),
        )
    }
}

fn offset_call(calling_frame: SymbolRef, value: Value) -> Value {
    match value {
        Value::Memory(data) => Value::Memory(MemoryData::new(
            data.base_type.clone(),
            match data.location {
                Location::FrameOffset(frame, offset) => Location::FrameOffsetBeforeCall(frame, calling_frame, offset),
                _ => data.location,
            },
            None,
        )),
        _ => value,
    }
}

fn convert_location(frame: SymbolRef, input: &symbol_table::Location) -> Location {
    match *input {
        symbol_table::Location::UndeterminedGlobal => unreachable!(),
        symbol_table::Location::Global(addr) => Location::Global(addr),
        symbol_table::Location::FrameOffset(offset) => Location::FrameOffset(frame, offset),
    }
}
