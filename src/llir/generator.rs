use std::sync::{Arc, RwLock};
use error;
use ir;
use llir::builder::RunBuilder;
use llir::common::convert_location;
use llir::{binop, AddToDataStackPointerData, BranchIfZeroData, CopyData, FrameBlock, GoToData, ImmediateValue,
           InlineAsmData, JumpRoutineData, Location, MemoryData, ReturnData, RunBlock, SPOffset, Statement, Value};
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
            Arc::clone(&irblock.symbol_table),
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
    symbol_table: Arc<RwLock<SymbolTable>>,
    frame_ref: SymbolRef,
    input: &[ir::Statement],
) -> error::Result<Vec<RunBlock>> {
    let mut run_builder = RunBuilder::new(Arc::clone(&symbol_table));
    for irstmt in input {
        match *irstmt {
            ir::Statement::Assign(ref data) => {
                let right_value = resolve_expr_to_value(&mut run_builder, frame_ref, &data.right_value)?;
                let left_location = resolve_expr_to_location(&mut run_builder, frame_ref, &data.left_value)?;

                generate_copy(
                    &mut run_builder,
                    data.tag,
                    data.value_type.as_ref().unwrap(),
                    right_value,
                    left_location,
                )?;
            }
            ir::Statement::Call(ref data) => {
                generate_function_call(&mut run_builder, frame_ref, data)?;
            }
            ir::Statement::Conditional(ref data) => {
                let _start_condition_block_ref = run_builder.new_block().block_ref;
                let condition = resolve_expr_to_value(&mut run_builder, frame_ref, &data.condition)?;
                let end_condition_block_ref = run_builder.current_block().block_ref;

                let true_block_ref = run_builder.append_blocks(generate_runs(
                    Arc::clone(&symbol_table),
                    frame_ref,
                    &data.when_true,
                )?);
                let false_block_ref = run_builder.append_blocks(generate_runs(
                    Arc::clone(&symbol_table),
                    frame_ref,
                    &data.when_false,
                )?);
                let after_both_block_ref = run_builder.new_block().symbol();

                run_builder.block(true_block_ref).add_statement(Statement::GoTo(
                    GoToData::new(data.tag, after_both_block_ref),
                ));

                let false_block_symbol = run_builder.block(false_block_ref).symbol();
                run_builder.block(end_condition_block_ref).add_statement(
                    Statement::BranchIfZero(
                        BranchIfZeroData::new(
                            data.tag,
                            condition,
                            false_block_symbol,
                        ),
                    ),
                );
                run_builder.new_block();
            }
            ir::Statement::InlineAsm(ref data) => {
                run_builder.current_block().add_statement(Statement::InlineAsm(
                    InlineAsmData::new(data.tag, Arc::clone(&data.asm)),
                ));
            }
            ir::Statement::WhileLoop(ref data) => {
                let start_condition_block_ref = run_builder.new_block().block_ref;
                let start_condition_block_symbol = run_builder.block(start_condition_block_ref).symbol();
                let condition = resolve_expr_to_value(&mut run_builder, frame_ref, &data.condition)?;
                let end_condition_block_ref = run_builder.current_block().block_ref;
                let body_block_ref = run_builder.append_blocks(generate_runs(
                    Arc::clone(&symbol_table),
                    frame_ref,
                    &data.body,
                )?);
                let after_body_block_symbol = run_builder.new_block().symbol();

                {
                    let mut end_condition_block = run_builder.block(end_condition_block_ref);
                    end_condition_block.add_statement(Statement::BranchIfZero(BranchIfZeroData::new(
                        data.tag,
                        condition,
                        after_body_block_symbol,
                    )));
                }

                run_builder.block(body_block_ref).add_statement(Statement::GoTo(
                    GoToData::new(data.tag, start_condition_block_symbol),
                ));
                run_builder.new_block();
            }
            ir::Statement::Return(ref data) => {
                if let Some(ref expr) = data.value {
                    let value = resolve_expr_to_value(&mut run_builder, frame_ref, expr)?;
                    generate_copy(
                        &mut run_builder,
                        data.tag,
                        data.value_type.as_ref().unwrap(),
                        value,
                        RETURN_LOCATION_LO,
                    )?;
                }
                run_builder.current_block().add_statement(
                    Statement::Return(ReturnData::new(data.tag)),
                );
            }
            ir::Statement::GoTo(ref data) => {
                if let Some(symbol_ref) = symbol_table.read().unwrap().find_symbol(&data.destination) {
                    run_builder.current_block().add_statement(
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

    Ok(run_builder.build())
}

fn generate_copy(
    run_builder: &mut RunBuilder,
    tag: SrcTag,
    value_type: &BaseType,
    value: Value,
    destination: Location,
) -> error::Result<()> {
    let mut block = run_builder.current_block();
    match *value_type {
        BaseType::Bool | BaseType::U8 => {
            block.add_statement(Statement::Copy(CopyData::new(tag, destination, value)));
        }
        BaseType::U16 |
        BaseType::Pointer(_) => {
            block.add_statement(Statement::Copy(CopyData::new(
                tag,
                destination.high_byte(),
                Value::high_byte(&value),
            )));
            block.add_statement(Statement::Copy(CopyData::new(
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
    run_builder: &mut RunBuilder,
    frame_ref: SymbolRef,
    expr: &ir::Expr,
) -> error::Result<Location> {
    match resolve_expr_to_value(run_builder, frame_ref, expr)? {
        Value::Immediate(_, _) => Err(error::ErrorKind::InvalidLeftValue(expr.src_tag()).into()),
        Value::Memory(data) => Ok(data.location),
    }
}

fn resolve_expr_to_value(run_builder: &mut RunBuilder, frame_ref: SymbolRef, expr: &ir::Expr) -> error::Result<Value> {
    let symbol_table = Arc::clone(run_builder.symbol_table());
    match *expr {
        ir::Expr::ArrayIndex(ref data) => {
            let array = symbol_table.write().unwrap().variable(data.array).unwrap();
            let array_name = symbol_table.write().unwrap().get_symbol_name(data.array).unwrap();
            let index_value = resolve_expr_to_value(run_builder, frame_ref, &data.index)?;
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
                    // Copy the pointer address to a new temporary
                    let addr = convert_location(
                        frame_ref,
                        &symbol_table.write().unwrap().create_temporary_location(
                            data.array_type.as_ref().unwrap(),
                        ),
                    );
                    generate_copy(
                        run_builder,
                        data.tag,
                        data.array_type.as_ref().unwrap(),
                        Value::Memory(MemoryData::new(
                            data.array_type.as_ref().unwrap().clone(),
                            convert_location(frame_ref, &array.location),
                            Some(Arc::new(format!("{}[]", array_name))),
                        )),
                        addr.clone(),
                    )?;

                    // Add the index to the pointer temporary
                    binop::BinopGenerator::new(
                        run_builder,
                        frame_ref,
                        data.tag,
                        &BaseType::U16,
                        &addr,
                        &Value::Memory(MemoryData::new(
                            BaseType::U16,
                            addr.clone(),
                            Some(Arc::new(format!("tmp_{}[]", array_name))),
                        )),
                        &index_value,
                    ).generate(ast::BinaryOperator::Add)?;

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
            if let Some(ref variable) = symbol_table.read().unwrap().variable(data.symbol) {
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
                                Some(
                                    symbol_table
                                        .read()
                                        .unwrap()
                                        .get_symbol_name(data.symbol)
                                        .unwrap(),
                                ),
                            ))),
                        }
                    }
                    _ => Ok(Value::Memory(MemoryData::new(
                        variable.base_type.clone(),
                        convert_location(frame_ref, &variable.location),
                        Some(
                            symbol_table
                                .read()
                                .unwrap()
                                .get_symbol_name(data.symbol)
                                .unwrap(),
                        ),
                    ))),
                }
            } else if let Some(ref constant) = symbol_table.read().unwrap().constant(data.symbol) {
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
                &symbol_table.write().unwrap().create_temporary_location(dest_type),
            );

            let left_value = resolve_expr_to_value(run_builder, frame_ref, &*data.left)?;
            let right_value = resolve_expr_to_value(run_builder, frame_ref, &*data.right)?;

            binop::BinopGenerator::new(
                run_builder,
                frame_ref,
                data.tag,
                dest_type,
                &dest,
                &left_value,
                &right_value,
            ).generate(data.op)?;

            Ok(Value::Memory(MemoryData::new(BaseType::U8, dest, None)))
        }
        ir::Expr::Call(ref data) => generate_function_call(run_builder, frame_ref, data),
    }
}

fn generate_function_call(
    run_builder: &mut RunBuilder,
    frame_ref: SymbolRef,
    call_data: &ir::CallData,
) -> error::Result<Value> {
    let symbol_table = Arc::clone(run_builder.symbol_table());
    let optional_function_ref = symbol_table.read().unwrap().find_symbol(&call_data.function);
    if let Some(function_ref) = optional_function_ref {
        let function = symbol_table.read().unwrap().function_by_name(&call_data.function).unwrap();
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
            argument_values.push(resolve_expr_to_value(run_builder, frame_ref, argument)?)
        }
        run_builder.current_block().add_statement(Statement::AddToDataStackPointer(
            AddToDataStackPointerData::new(
                call_data.tag,
                SPOffset::FrameSize(function_ref),
            ),
        ));
        if !metadata.parameters.is_empty() {
            let mut frame_offset = 0;
            for (i, argument_value) in argument_values.into_iter().enumerate() {
                generate_copy(
                    run_builder,
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
        run_builder.current_block().add_statement(
            Statement::JumpRoutine(JumpRoutineData::new(
                call_data.tag,
                Location::UnresolvedGlobal(function_ref),
            )),
        );

        // Restore the stack pointer
        run_builder.current_block().add_statement(Statement::AddToDataStackPointer(
            AddToDataStackPointerData::new(
                call_data.tag,
                SPOffset::NegativeFrameSize(function_ref),
            ),
        ));

        if call_data.return_type.as_ref().unwrap().size().is_some() {
            let dest = convert_location(
                frame_ref,
                &symbol_table.write().unwrap().create_temporary_location(&BaseType::U8),
            );
            generate_copy(
                run_builder,
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
