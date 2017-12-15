use base_type::BaseType;
use error;
use llir::{BranchFlag, CompareBranchData, CopyData, GoToData, ImmediateValue, Statement, Value};
use parse::ast::BinaryOperator;
use super::{BinopGenerator, TypeCoercer};

#[derive(new)]
pub struct CompareGenerator<'a> {
    binop: BinopGenerator<'a>,
}

impl<'a> CompareGenerator<'a> {
    pub fn generate(mut self, op: BinaryOperator) -> error::Result<()> {
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

            self.generate_op(actual_left.as_ref(), actual_right.as_ref(), false, op)
        } else if actual_left.value_type() == BaseType::U8 {
            if *self.binop.dest_type != BaseType::U8 {
                panic!("type checking didn't catch coersion of U8 to U16");
            }

            self.generate_op(actual_left.as_ref(), actual_right.as_ref(), true, op)
        } else {
            panic!("something went wrong with type checking");
        }
    }

    fn generate_op(&mut self, left: &Value, right: &Value, single_byte: bool, op: BinaryOperator) -> error::Result<()> {
        use parse::ast::BinaryOperator::*;
        match op {
            LessThan => self.generate_cmp(left, right, single_byte, BranchFlag::Carry, false),
            GreaterThan => self.generate_cmp(right, left, single_byte, BranchFlag::Carry, false),
            LessThanEqual => self.generate_cmp(right, left, single_byte, BranchFlag::Carry, true),
            GreaterThanEqual => self.generate_cmp(left, right, single_byte, BranchFlag::Carry, true),
            Equal => self.generate_cmp(left, right, single_byte, BranchFlag::Zero, true),
            NotEqual => self.generate_cmp(left, right, single_byte, BranchFlag::Zero, false),
            _ => unreachable!(),
        }
    }

    fn generate_cmp(
        &mut self,
        left: &Value,
        right: &Value,
        single_byte: bool,
        flag: BranchFlag,
        set_value: bool,
    ) -> error::Result<()> {
        if single_byte {
            self.generate_8bit_cmp(left, right, flag, set_value)
        } else {
            self.generate_16bit_cmp(left, right, flag, set_value)
        }
    }

    fn generate_8bit_cmp(
        &mut self,
        left: &Value,
        right: &Value,
        flag: BranchFlag,
        set_value: bool,
    ) -> error::Result<()> {
        let compare_block_ref = self.binop.run_builder.new_block().block_ref;
        let clear_block_ref = self.binop.run_builder.new_block().block_ref;
        let set_block_ref = self.binop.run_builder.new_block().block_ref;
        let set_block_symbol = self.binop.run_builder.block(set_block_ref).symbol();
        let after_block_symbol = self.binop.run_builder.new_block().symbol();

        self.binop.run_builder.block(compare_block_ref).add_statement(
            Statement::CompareBranch(
                CompareBranchData::new(
                    self.binop.src_tag,
                    left.clone(),
                    right.clone(),
                    flag,
                    Some(set_block_symbol),
                    None,
                ),
            ),
        );

        self.binop
            .run_builder
            .block(clear_block_ref)
            .add_statement(Statement::Copy(CopyData::new(
                self.binop.src_tag,
                self.binop.dest.clone(),
                imm_bool(!set_value),
            )))
            .add_statement(Statement::GoTo(
                GoToData::new(self.binop.src_tag, after_block_symbol),
            ));

        self.binop.run_builder.block(set_block_ref).add_statement(
            Statement::Copy(CopyData::new(
                self.binop.src_tag,
                self.binop.dest.clone(),
                imm_bool(set_value),
            )),
        );

        Ok(())
    }

    fn generate_16bit_cmp(
        &mut self,
        left: &Value,
        right: &Value,
        flag: BranchFlag,
        set_value: bool,
    ) -> error::Result<()> {
        let first_compare_block_ref = self.binop.run_builder.new_block().block_ref;
        let second_compare_block_ref = self.binop.run_builder.new_block().block_ref;
        let set_block_ref = self.binop.run_builder.new_block().block_ref;
        let clear_block_ref = self.binop.run_builder.new_block().block_ref;
        let clear_block_symbol = self.binop.run_builder.block(clear_block_ref).symbol();
        let after_block_symbol = self.binop.run_builder.new_block().symbol();

        self.binop.run_builder.block(first_compare_block_ref).add_statement(
            Statement::CompareBranch(
                CompareBranchData::new(
                    self.binop.src_tag,
                    Value::high_byte(left),
                    Value::high_byte(right),
                    flag,
                    None,
                    Some(clear_block_symbol),
                ),
            ),
        );

        self.binop.run_builder.block(second_compare_block_ref).add_statement(
            Statement::CompareBranch(
                CompareBranchData::new(
                    self.binop.src_tag,
                    Value::low_byte(left),
                    Value::low_byte(right),
                    flag,
                    None,
                    Some(clear_block_symbol),
                ),
            ),
        );

        self.binop
            .run_builder
            .block(set_block_ref)
            .add_statement(Statement::Copy(CopyData::new(
                self.binop.src_tag,
                self.binop.dest.clone(),
                imm_bool(set_value),
            )))
            .add_statement(Statement::GoTo(
                GoToData::new(self.binop.src_tag, after_block_symbol),
            ));

        self.binop.run_builder.block(clear_block_ref).add_statement(
            Statement::Copy(CopyData::new(
                self.binop.src_tag,
                self.binop.dest.clone(),
                imm_bool(!set_value),
            )),
        );

        Ok(())
    }
}

fn imm_bool(value: bool) -> Value {
    Value::Immediate(BaseType::U8, ImmediateValue::Number(value as i32))
}
