//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use base_type::BaseType;
use error;
use llir::{BinaryOpData, CarryMode, Statement, Value};
use super::{BinopGenerator, TypeCoercer};

#[derive(new)]
pub struct AddGenerator<'a> {
    binop: BinopGenerator<'a>,
}

impl<'a> AddGenerator<'a> {
    pub fn generate(mut self, subtract: bool) -> error::Result<()> {
        let (actual_left, actual_right) = {
            let mut type_coercer = TypeCoercer::new(
                self.binop.run_builder,
                self.binop.frame_ref,
                self.binop.src_tag,
            );
            type_coercer.coerce_values_to_same_types(
                self.binop.left_value,
                self.binop.right_value,
            )?
        };

        if actual_left.value_type() == BaseType::U16 {
            if *self.binop.dest_type != BaseType::U16 {
                panic!("type checking didn't catch coersion of U16 to U8");
            }

            self.generate_16x16_into_16(actual_left.as_ref(), actual_right.as_ref(), subtract)
        } else if actual_left.value_type() == BaseType::U8 {
            if *self.binop.dest_type != BaseType::U8 {
                panic!("type checking didn't catch coersion of U8 to U16");
            }

            self.generate_8x8_into_8(actual_left.as_ref(), actual_right.as_ref(), subtract)
        } else {
            panic!("something went wrong with type checking");
        }
    }

    fn generate_16x16_into_16(&mut self, left_value: &Value, right_value: &Value, subtract: bool) -> error::Result<()> {
        let first_op = BinaryOpData::new(
            self.binop.src_tag,
            self.binop.dest.low_byte(),
            Value::low_byte(left_value),
            Value::low_byte(right_value),
            if subtract {
                CarryMode::SetCarry
            } else {
                CarryMode::ClearCarry
            },
        );
        let second_op = BinaryOpData::new(
            self.binop.src_tag,
            self.binop.dest.high_byte(),
            Value::high_byte(left_value),
            Value::high_byte(right_value),
            CarryMode::DontCare,
        );
        if subtract {
            self.binop
                .run_builder
                .current_block()
                .add_statement(Statement::Subtract(first_op))
                .add_statement(Statement::Subtract(second_op));
        } else {
            self.binop
                .run_builder
                .current_block()
                .add_statement(Statement::Add(first_op))
                .add_statement(Statement::Add(second_op));
        }
        Ok(())
    }

    fn generate_8x8_into_8(&mut self, left_value: &Value, right_value: &Value, subtract: bool) -> error::Result<()> {
        let bin_op_data = BinaryOpData::new(
            self.binop.src_tag,
            self.binop.dest.clone(),
            left_value.clone(),
            right_value.clone(),
            if subtract {
                CarryMode::SetCarry
            } else {
                CarryMode::ClearCarry
            },
        );
        self.binop.run_builder.current_block().add_statement(if subtract {
            Statement::Subtract(bin_op_data)
        } else {
            Statement::Add(bin_op_data)
        });
        Ok(())
    }
}
