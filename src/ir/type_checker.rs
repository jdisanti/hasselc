use num_traits::*;

use error::{self, ErrorKind};
use ir::block::*;
use src_tag::SrcTag;
use symbol_table::{SymbolRef, SymbolTable};
use types::{Type, TypedValue};

pub fn resolve_types(blocks: &mut Vec<Block>) -> error::Result<()> {
    for block in blocks {
        resolve_statements(
            &block.symbol_table.read().unwrap(),
            block.metadata.read().unwrap().return_type,
            &mut block.body,
        )?;
    }
    Ok(())
}

fn resolve_statements(
    symbol_table: &SymbolTable,
    return_type: Type,
    statements: &mut Vec<Statement>,
) -> error::Result<()> {
    for mut statement in statements {
        resolve_statement(symbol_table, return_type, &mut statement)?;
    }
    Ok(())
}

fn resolve_statement(symbol_table: &SymbolTable, return_type: Type, statement: &mut Statement) -> error::Result<()> {
    use ir::block::Statement::*;

    match *statement {
        Assign(ref mut data) => if let Some(variable) = symbol_table.variable(&data.symbol) {
            resolve_expression(symbol_table, variable.type_name, &mut data.value)?;
        } else {
            return Err(ErrorKind::SymbolNotFound(data.tag, SymbolRef::clone(&data.symbol)).into());
        },
        Call(ref mut data) => resolve_call(symbol_table, data)?,
        Conditional(ref mut data) => {
            resolve_expression(symbol_table, Type::U8, &mut data.condition)?;
            resolve_statements(symbol_table, return_type, &mut data.when_true)?;
            resolve_statements(symbol_table, return_type, &mut data.when_false)?;
        }
        Return(ref mut data) => if let Some(ref mut value) = data.value {
            resolve_expression(symbol_table, return_type, value)?;
        } else if return_type != Type::Void {
            return Err(ErrorKind::MustReturnAValue(data.tag).into());
        },
        WhileLoop(ref mut data) => {
            resolve_expression(symbol_table, Type::U8, &mut data.condition)?;
            resolve_statements(symbol_table, return_type, &mut data.body)?;
        }
        Break | GoTo(_) => {}
    }

    Ok(())
}

fn resolve_expression(symbol_table: &SymbolTable, required_type: Type, expression: &mut Expr) -> error::Result<()> {
    use ir::block::Expr::*;

    match *expression {
        Number(ref mut data) => {
            let value = if let TypedValue::UnresolvedInt(val) = data.value {
                val
            } else {
                return Err(ErrorKind::TypeError(data.tag, required_type, data.value.get_type()).into());
            };
            data.value = match required_type {
                Type::U8 => TypedValue::U8(bounds_check(data.tag, value)?),
                Type::U16 => TypedValue::U16(bounds_check(data.tag, value)?),
                Type::Unresolved | Type::Void => unreachable!(),
            };
        }
        Symbol(ref data) => match symbol_table.type_of(&data.name) {
            Some(typ) => if required_type != typ {
                return Err(ErrorKind::TypeError(data.tag, required_type, typ).into());
            },
            None => {
                println!("symbol_table: {:#?}", symbol_table);
                return Err(ErrorKind::SymbolNotFound(data.tag, SymbolRef::clone(&data.name)).into());
            }
        },
        BinaryOp(ref mut data) => {
            resolve_expression(symbol_table, required_type, &mut *data.left)?;
            resolve_expression(symbol_table, required_type, &mut *data.right)?;
        }
        Call(ref mut data) => {
            resolve_call(symbol_table, data)?;
            match symbol_table.type_of(&data.function) {
                Some(typ) => if required_type != typ {
                    return Err(ErrorKind::TypeError(data.tag, required_type, typ).into());
                },
                None => return Err(ErrorKind::SymbolNotFound(data.tag, SymbolRef::clone(&data.function)).into()),
            }
        }
    }

    Ok(())
}

fn resolve_call(symbol_table: &SymbolTable, call_data: &mut CallData) -> error::Result<()> {
    if let Some(function) = symbol_table.function(&call_data.function) {
        let arguments = &function.read().unwrap().parameters;
        if arguments.len() != call_data.arguments.len() {
            return Err(
                ErrorKind::ExpectedNArgumentsGotM(
                    call_data.tag,
                    SymbolRef::clone(&call_data.function),
                    arguments.len(),
                    call_data.arguments.len(),
                ).into(),
            );
        }
        for (index, argument) in arguments.iter().enumerate() {
            resolve_expression(
                symbol_table,
                argument.type_name,
                &mut call_data.arguments[index],
            )?;
        }
        Ok(())
    } else {
        Err(ErrorKind::SymbolNotFound(call_data.tag, SymbolRef::clone(&call_data.function)).into())
    }
}

fn bounds_check<N: ::num_traits::PrimInt>(tag: SrcTag, value: i32) -> error::Result<N> {
    let unsigned_val = value as u32;
    if unsigned_val > <u32 as NumCast>::from(N::max_value()).unwrap()
        || unsigned_val < <u32 as NumCast>::from(N::min_value()).unwrap()
    {
        Err(
            ErrorKind::OutOfBounds(
                tag,
                value as isize,
                <isize as NumCast>::from(N::min_value()).unwrap(),
                <isize as NumCast>::from(N::max_value()).unwrap(),
            ).into(),
        )
    } else {
        Ok(N::from(unsigned_val).unwrap())
    }
}
