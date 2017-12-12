use error;
use llir::{BinaryOpData, CarryMode, Location, Statement, Value};
use src_tag::SrcTag;
use type_expr::BaseType;

pub fn generate_add(
    statements: &mut Vec<Statement>,
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
) -> error::Result<()> {
    generate(
        statements,
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
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
) -> error::Result<()> {
    generate(
        statements,
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
    src_tag: SrcTag,
    dest_type: &BaseType,
    dest: &Location,
    left_value: &Value,
    right_value: &Value,
    subtract: bool,
) -> error::Result<()> {
    let left_type = left_value.value_type();
    let right_type = left_value.value_type();

    if left_type == right_type && *dest_type == left_type {
        if let BaseType::U8 = left_type {
            generate_8x8_into_8(statements, src_tag, dest, left_value, right_value, subtract)
        } else if let BaseType::U16 = left_type {
            generate_16x16_into_16(statements, src_tag, dest, left_value, right_value, subtract)
        } else {
            unreachable!()
        }
    } else {
        unimplemented!("mixed type operations")
    }
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
