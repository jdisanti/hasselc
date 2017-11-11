use std::rc::Rc;
use std::cell::RefCell;
use ast;
use ir;

pub fn generate_ir(input: &Vec<ast::Expression>) -> Result<Vec<ir::IR>, ()> {
    let global_symbol_table = Rc::new(RefCell::new(ir::SymbolTable::new()));
    let mut blocks: Vec<ir::IR> = vec![ir::IR::new_anonymous_block(global_symbol_table.clone())];

    for ast_expr in input {
        match *ast_expr {
            ast::Expression::DeclareFunction { ref name, ref parameters, ref return_type, ref body } => {
                let mut function = ir::IR::new_function_block(global_symbol_table.clone(),
                    name.clone(), parameters.clone(), return_type.clone());


                if blocks.last_mut().unwrap().is_empty_anonymous() {
                    let old_block = blocks.pop().unwrap();
                    if let &Some(ref location) = old_block.location() {
                        function.set_location(*location);
                    }
                }

                // TODO: error if global symbol table already has this name
                let metadata = ir::FunctionMetadata {
                    location: *function.location(),
                    parameters: parameters.clone(),
                    return_type: return_type.clone(),
                };
                global_symbol_table.borrow_mut().functions.insert(ir::SymbolRef(name.clone()), Rc::new(metadata));

                let symbol_table = function.symbol_table();
                let body_ir = generate_statement_irs(&mut *symbol_table.borrow_mut(), body)?;
                function.body_mut().extend(body_ir);
                blocks.push(function);

            },
            ast::Expression::Org { ref org } => {
                blocks.last_mut().unwrap().set_location(ir::Location::Global(*org as u16));
            },
            ast::Expression::Comment => { },
            ast::Expression::Error => unreachable!("error"),
            ast::Expression::BinaryOp { .. } => unreachable!("binary_op"),
            ast::Expression::Number(_) => unreachable!("number"),
            ast::Expression::Name(_) => unreachable!("name"),
            _ => {
                let stmt = generate_statement_ir(&mut *global_symbol_table.borrow_mut(), ast_expr)?;
                blocks.last_mut().unwrap().body_mut().extend(stmt);
            },
        }
    }

    Ok(blocks)
}

fn generate_statement_irs(symbol_table: &mut ir::SymbolTable, input: &Vec<ast::Expression>) -> Result<Vec<ir::Statement>, ()> {
    let mut statements: Vec<ir::Statement> = vec![];

    for ast_expr in input {
        statements.extend(generate_statement_ir(symbol_table, ast_expr)?);
    }

    Ok(statements)
}

fn generate_statement_ir(symbol_table: &mut ir::SymbolTable, input: &ast::Expression) -> Result<Vec<ir::Statement>, ()> {
    let mut statements: Vec<ir::Statement> = vec![];
    match *input {
        ast::Expression::Assignment { ref name, ref value } => {
            let symbol_ref = ir::SymbolRef(name.clone());
            if symbol_table.variable(&symbol_ref).is_some() {
                statements.push(ir::Statement::Assign {
                    symbol: symbol_ref,
                    value: generate_expression(value),
                });
            } else {
                // TODO: symbol not found error
                unimplemented!()
            }
        },
        ast::Expression::Break => {
            // TODO
        },
        ast::Expression::CallFunction { ref name, ref arguments } => {
            let stmt = ir::Statement::Call(ir::Expr::Call {
                symbol: ir::SymbolRef(name.clone()),
                arguments: generate_expressions(arguments),
            });
            statements.push(stmt);
        },
        ast::Expression::DeclareConst { ref name_type, ref value } => {
            // TODO
        },
        ast::Expression::DeclareVariable { ref name_type, ref value } => {
            let symbol_ref = ir::SymbolRef(name_type.name.clone());
            if symbol_table.variable(&symbol_ref).is_some() {
                // TODO: duplicate symbol error
                unimplemented!()
            }
            let next_location = symbol_table.next_local_stack_offset(name_type.type_name.size());
            symbol_table.variables.insert(symbol_ref.clone(),
                (name_type.type_name, ir::Location::StackOffset(next_location)));
            let assignment = ir::Statement::Assign {
                symbol: symbol_ref,
                value: generate_expression(value),
            };
            statements.push(assignment);
        },
        ast::Expression::LeftShift(ref name) => {
            // TODO
        },
        ast::Expression::RotateLeft(ref name) => {
            // TODO
        },
        ast::Expression::RotateRight(ref name) => {
            // TODO
        },
        ast::Expression::Return { ref value } => {
            statements.push(ir::Statement::Return(generate_expression(value)));
        },
        ast::Expression::Comment => { },
        ast::Expression::BinaryOp { .. } => unreachable!("binary_op"),
        ast::Expression::DeclareFunction { .. } => unreachable!("declare_function"),
        ast::Expression::Error => unreachable!("error"),
        ast::Expression::Name(_) => unreachable!("name"),
        ast::Expression::Number(_) => unreachable!("number"),
        ast::Expression::Org { .. } => unreachable!("org"),
    }

    Ok(statements)
}

fn generate_expressions(input: &Vec<ast::Expression>) -> Vec<ir::Expr> {
    let mut result = Vec::new();
    for expression in input {
        result.push(generate_expression(expression));
    }
    result
}

fn generate_expression(input: &ast::Expression) -> ir::Expr {
    match *input {
        ast::Expression::BinaryOp { ref left, ref op, ref right } => {
            ir::Expr::BinaryOp {
                op: *op,
                left: Box::new(generate_expression(left)),
                right: Box::new(generate_expression(right)),
            }
        },
        ast::Expression::Name(ref name) => {
            ir::Expr::Symbol(ir::SymbolRef(name.clone()))
        },
        ast::Expression::Number(num) => {
            ir::Expr::Number(num)
        },
        ast::Expression::CallFunction { ref name, ref arguments } => {
            let fn_symbol_ref = ir::SymbolRef(name.clone());
            let args = generate_expressions(arguments);
            ir::Expr::Call {
                symbol: fn_symbol_ref,
                arguments: args,
            }
        },
        _ => panic!("not an expression: {:?}", input)
    }
}
