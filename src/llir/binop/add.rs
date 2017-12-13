use std::borrow::Cow;
use std::sync::Arc;

use error;
use llir::{BinaryOpData, CarryMode, CopyData, ImmediateValue, MemoryData, Location, Statement, Value};
use llir::common::convert_location;
use src_tag::SrcTag;
use base_type::BaseType;
use symbol_table::{SymbolRef, SymbolTable};

pub fn generate_add(
    statements: &mut Vec<Statement>,
    symbol_table: &mut SymbolTable,
    frame_ref: SymbolRef,
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
) -> error::Result<()> {
    generate(
        statements,
        symbol_table,
        frame_ref,
        src_tag,
        dest_type,
        dest,
        left_value,
        right_value,
        false,
    )
}

pub fn generate_sub(
    statements: &mut Vec<Statement>,
    symbol_table: &mut SymbolTable,
    frame_ref: SymbolRef,
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
) -> error::Result<()> {
    generate(
        statements,
        symbol_table,
        frame_ref,
        src_tag,
        dest_type,
        dest,
        left_value,
        right_value,
        true,
    )
}

fn generate(
    statements: &mut Vec<Statement>,
    symbol_table: &mut SymbolTable,
    frame_ref: SymbolRef,
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
    subtract: bool,
) -> error::Result<()> {
    let left_type = left_value.value_type();
    let right_type = right_value.value_type();

    let mut actual_left = Cow::Borrowed(left_value);
    let mut actual_right = Cow::Borrowed(right_value);

    if *dest_type == BaseType::U16 {
        if left_type == BaseType::U8 {
            let temp_addr = convert_location(
                frame_ref,
                &symbol_table.create_temporary_location(&BaseType::U16),
            );
            convert_u8_into_u16(statements, src_tag, &temp_addr, left_value)?;
            actual_left = Cow::Owned(Value::Memory(MemoryData::new(
                BaseType::U16,
                temp_addr,
                Some(Arc::new("tmp_u8_to_u16_L".into())),
            )));
        }
        if right_type == BaseType::U8 {
            let temp_addr = convert_location(
                frame_ref,
                &symbol_table.create_temporary_location(&BaseType::U16),
            );
            convert_u8_into_u16(statements, src_tag, &temp_addr, right_value)?;
            actual_right = Cow::Owned(Value::Memory(MemoryData::new(
                BaseType::U16,
                temp_addr,
                Some(Arc::new("tmp_u8_to_u16_R".into())),
            )));
        }
        generate_16x16_into_16(
            statements,
            src_tag,
            dest,
            actual_left.as_ref(),
            actual_right.as_ref(),
            subtract,
        )
    } else {
        if left_type == BaseType::U16 || right_type == BaseType::U16 {
            panic!(
                "something went wrong in type checking... \
                shouldn't be able to add u16s and store into u8s without casting"
            );
        }
        generate_8x8_into_8(
            statements,
            src_tag,
            dest,
            actual_left.as_ref(),
            actual_right.as_ref(),
            subtract,
        )
    }
}

fn convert_u8_into_u16(
    statements: &mut Vec<Statement>,
    src_tag: SrcTag,
    dest: &Location,
    value: &Value,
) -> error::Result<()> {
    statements.push(Statement::Copy(CopyData::new(
        src_tag,
        dest.high_byte(),
        Value::Immediate(BaseType::U8, ImmediateValue::Number(0)),
    )));
    statements.push(Statement::Copy(
        CopyData::new(src_tag, dest.low_byte(), value.clone()),
    ));
    Ok(())
}

fn generate_16x16_into_16(
    statements: &mut Vec<Statement>,
    src_tag: SrcTag,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
    subtract: bool,
) -> error::Result<()> {
    let first_op = BinaryOpData::new(
        src_tag,
        dest.low_byte(),
        Value::low_byte(left_value),
        Value::low_byte(right_value),
        if subtract {
            CarryMode::SetCarry
        } else {
            CarryMode::ClearCarry
        },
    );
    let second_op = BinaryOpData::new(
        src_tag,
        dest.high_byte(),
        Value::high_byte(left_value),
        Value::high_byte(right_value),
        CarryMode::DontCare,
    );
    if subtract {
        statements.push(Statement::Subtract(first_op));
        statements.push(Statement::Subtract(second_op));
    } else {
        statements.push(Statement::Add(first_op));
        statements.push(Statement::Add(second_op));
    }
    Ok(())
}

fn generate_8x8_into_8(
    statements: &mut Vec<Statement>,
    src_tag: SrcTag,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
    subtract: bool,
) -> error::Result<()> {
    let bin_op_data = BinaryOpData::new(
        src_tag,
        dest.clone(),
        left_value.clone(),
        right_value.clone(),
        if subtract {
            CarryMode::SetCarry
        } else {
            CarryMode::ClearCarry
        },
    );
    statements.push(if subtract {
        Statement::Subtract(bin_op_data)
    } else {
        Statement::Add(bin_op_data)
    });
    Ok(())
}
