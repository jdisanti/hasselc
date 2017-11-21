use std::sync::{Arc, RwLock};
use ast;
use ir;
use symbol_table::{FunctionMetadata, Location, SymbolRef, SymbolTable, Variable};
use error::{self, ErrorKind};

pub fn generate_ir(input: &[ast::Expression]) -> error::Result<Vec<ir::Block>> {
    let global_symbol_table = Arc::new(RwLock::new(SymbolTable::new()));
    let mut blocks = vec![ir::Block::new_anonymous(Arc::clone(&global_symbol_table))];

    for ast_expr in input {
        match *ast_expr {
            ast::Expression::DeclareFunction(ref data) => {
                let location = if blocks.last_mut().unwrap().is_empty_anonymous() {
                    let old_block = blocks.pop().unwrap();
                    old_block.location
                } else {
                    None
                };

                let symbol_ref = SymbolRef::clone(&data.name);
                let metadata = Arc::new(RwLock::new(FunctionMetadata {
                    name: Arc::clone(&data.name),
                    location: location,
                    parameters: data.parameters.clone(),
                    return_type: data.return_type,
                    frame_size: 127, // 127 is an intentional non-sensical value
                }));

                let mut function = ir::Block::new_named(
                    data.tag,
                    Arc::clone(&global_symbol_table),
                    location,
                    Arc::clone(&metadata),
                )?;
                if !global_symbol_table
                    .write()
                    .unwrap()
                    .insert_function(Arc::clone(&symbol_ref), Arc::clone(&metadata))
                {
                    return Err(ErrorKind::DuplicateSymbol(data.tag, Arc::clone(&symbol_ref)).into());
                }

                let body_ir = generate_statement_irs(&mut *function.symbol_table.write().unwrap(), &data.body)?;
                function.body.extend(body_ir);
                blocks.push(function);
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
            let symbol_ref = SymbolRef::clone(&data.name);
            if symbol_table.variable(&symbol_ref).is_some() {
                statements.push(ir::Statement::Assign(ir::AssignData::new(
                    data.tag,
                    symbol_ref,
                    generate_expression(&data.value),
                )));
            } else {
                return Err(ErrorKind::SymbolNotFound(data.tag, Arc::clone(&data.name)).into());
            }
        }
        ast::Expression::Break => {
            unimplemented!("ir_gen: break");
        }
        ast::Expression::CallFunction(ref data) => {
            let stmt = ir::Statement::Call(ir::CallData::new(
                data.tag,
                ir::Expr::Call {
                    symbol: SymbolRef::clone(&data.name),
                    arguments: generate_expressions(&data.arguments),
                },
            ));
            statements.push(stmt);
        }
        ast::Expression::Conditional(ref data) => {
            let condition = generate_expression(&data.condition);
            let when_true = generate_statement_irs(symbol_table, &data.when_true)?;
            let when_false = generate_statement_irs(symbol_table, &data.when_false)?;
            statements.push(ir::Statement::Conditional(ir::ConditionalData::new(
                data.tag,
                condition,
                when_true,
                when_false,
            )));
        }
        ast::Expression::DeclareConst(ref _data) => {
            unimplemented!("ir_gen: declare const");
        }
        ast::Expression::DeclareVariable(ref data) => {
            let symbol_ref = SymbolRef::clone(&data.name_type.name);
            let next_location = symbol_table.next_frame_offset(data.name_type.type_name.size());
            let variable = Variable::new(
                data.name_type.type_name,
                Location::FrameOffset(next_location as i8),
            );
            if !symbol_table.insert_variable(SymbolRef::clone(&symbol_ref), variable) {
                return Err(ErrorKind::DuplicateSymbol(data.tag, SymbolRef::clone(&data.name_type.name)).into());
            }

            let assignment = ir::Statement::Assign(ir::AssignData::new(
                data.tag,
                symbol_ref,
                generate_expression(&data.value),
            ));
            statements.push(assignment);
        }
        ast::Expression::DeclareRegister(ref data) => {
            if data.location < 0 || data.location > 0xFFFF {
                return Err(ErrorKind::OutOfBounds(data.tag, data.location as isize, 0, 0xFFFF).into());
            }
            let symbol_ref = SymbolRef::clone(&data.name_type.name);
            let variable = Variable::new(
                data.name_type.type_name,
                Location::Global(data.location as u16),
            );
            if !symbol_table.insert_variable(symbol_ref, variable) {
                return Err(ErrorKind::DuplicateSymbol(data.tag, Arc::clone(&data.name_type.name)).into());
            }
        }
        ast::Expression::Return(ref data) => {
            statements.push(ir::Statement::Return(ir::ReturnData::new(
                data.tag,
                data.value.as_ref().map(|e| generate_expression(e)),
            )));
        }
        ast::Expression::GoTo(ref data) => {
            statements.push(ir::Statement::GoTo(
                ir::GoToData::new(data.tag, Arc::clone(&data.destination)),
            ));
        }
        ast::Expression::WhileLoop(ref data) => {
            let condition = generate_expression(&data.condition);
            let body = generate_statement_irs(symbol_table, &data.body)?;
            statements.push(ir::Statement::WhileLoop(
                ir::WhileLoopData::new(data.tag, condition, body),
            ));
        }
        ast::Expression::Comment => {}
        ast::Expression::BinaryOp { .. } => unreachable!("binary_op"),
        ast::Expression::DeclareFunction { .. } => unreachable!("declare_function"),
        ast::Expression::Error => unreachable!("error"),
        ast::Expression::Name(_) => unreachable!("name"),
        ast::Expression::Number(_) => unreachable!("number"),
        ast::Expression::Org { .. } => unreachable!("org"),
    }

    Ok(statements)
}

fn generate_expressions(input: &[ast::Expression]) -> Vec<ir::Expr> {
    let mut result = Vec::new();
    for expression in input {
        result.push(generate_expression(expression));
    }
    result
}

fn generate_expression(input: &ast::Expression) -> ir::Expr {
    match *input {
        ast::Expression::BinaryOp(ref data) => ir::Expr::BinaryOp {
            op: data.op,
            left: Box::new(generate_expression(&data.left)),
            right: Box::new(generate_expression(&data.right)),
        },
        ast::Expression::Name(ref data) => ir::Expr::Symbol(SymbolRef::clone(&data.name)),
        ast::Expression::Number(ref data) => ir::Expr::Number(data.value),
        ast::Expression::CallFunction(ref data) => {
            let fn_symbol_ref = SymbolRef::clone(&data.name);
            let args = generate_expressions(&data.arguments);
            ir::Expr::Call {
                symbol: fn_symbol_ref,
                arguments: args,
            }
        }
        _ => panic!("not an expression: {:?}", input),
    }
}
