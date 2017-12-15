use std::borrow::Cow;
use std::sync::Arc;

use base_type::BaseType;
use error;
use llir::builder::RunBuilder;
use llir::common::convert_location;
use llir::{CopyData, ImmediateValue, MemoryData, Location, Statement, Value};
use parse::ast::BinaryOperator;
use src_tag::SrcTag;
use symbol_table::SymbolRef;

pub mod add;
pub mod compare;

#[derive(new)]
pub struct BinopGenerator<'a> {
    run_builder: &'a mut RunBuilder,
    frame_ref: SymbolRef,
    src_tag: SrcTag,
    dest_type: &'a BaseType,
    dest: &'a Location,
    left_value: &'a Value,
    right_value: &'a Value,
}

impl<'a> BinopGenerator<'a> {
    pub fn generate(self, op: BinaryOperator) -> error::Result<()> {
        use parse::ast::BinaryOperator::*;
        match op {
            Add => add::AddGenerator::new(self).generate(false),
            Sub => add::AddGenerator::new(self).generate(true),
            Equal | NotEqual | LessThan | LessThanEqual | GreaterThan | GreaterThanEqual => {
                compare::CompareGenerator::new(self).generate(op)
            }
            _ => unimplemented!(),
        }
    }
}

#[derive(new)]
struct TypeCoercer<'a> {
    run_builder: &'a mut RunBuilder,
    frame_ref: SymbolRef,
    src_tag: SrcTag,
}

impl<'a> TypeCoercer<'a> {
    fn coerce_values_to_same_types<'b>(
        &mut self,
        left_value: &'b Value,
        right_value: &'b Value,
    ) -> error::Result<(Cow<'b, Value>, Cow<'b, Value>)> {
        let left_type = left_value.value_type();
        let right_type = right_value.value_type();

        let mut actual_left = Cow::Borrowed(left_value);
        let mut actual_right = Cow::Borrowed(right_value);

        if let Some(coerced_type) = BaseType::choose_type(&left_type, &right_type) {
            if coerced_type == BaseType::U16 {
                if left_type == BaseType::U8 {
                    actual_left = Cow::Owned(self.convert_u8_into_u16(left_value)?);
                }
                if right_type == BaseType::U8 {
                    actual_right = Cow::Owned(self.convert_u8_into_u16(right_value)?);
                }
            }
        }
        if actual_left.value_type() != actual_right.value_type() {
            panic!("something went wrong in type checking...")
        }

        Ok((actual_left, actual_right))
    }

    fn convert_u8_into_u16(&mut self, value: &Value) -> error::Result<Value> {
        let temp_addr = convert_location(
            self.frame_ref,
            &self.run_builder.symbol_table().write().unwrap().create_temporary_location(
                &BaseType::U16,
            ),
        );
        self.run_builder.current_block().add_statement(Statement::Copy(CopyData::new(
            self.src_tag,
            temp_addr.high_byte(),
            Value::Immediate(BaseType::U8, ImmediateValue::Number(0)),
        )));
        self.run_builder.current_block().add_statement(Statement::Copy(CopyData::new(
            self.src_tag,
            temp_addr.low_byte(),
            value.clone(),
        )));
        Ok(Value::Memory(MemoryData::new(
            BaseType::U16,
            temp_addr,
            Some(Arc::new("tmp_u8_to_u16".into())),
        )))
    }
}
