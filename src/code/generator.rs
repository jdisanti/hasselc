use std::sync::Arc;
use code::{Code, CodeBlock, Global, Parameter};
use code::register::{Register, RegisterAllocator};
use error;
use llir;
use symbol_table::{SymbolName, SymbolRef};
use src_tag::{SrcTag, SrcTagged};
use src_unit::SrcUnits;
use base_type::BaseType;

pub struct CodeBlockGenerator<'a> {
    llir_blocks: &'a [llir::FrameBlock],
    code_blocks: Vec<CodeBlock>,
    src_units: &'a SrcUnits,
}

impl<'a> CodeBlockGenerator<'a> {
    pub fn new<'b>(src_units: &'b SrcUnits, input: &'b [llir::FrameBlock]) -> CodeBlockGenerator<'b> {
        CodeBlockGenerator {
            llir_blocks: input,
            code_blocks: Vec::new(),
            src_units: src_units,
        }
    }

    pub fn generate(mut self) -> error::Result<Vec<CodeBlock>> {
        for frame_block in self.llir_blocks {
            self.code_blocks.push(CodeBlock::new(
                SymbolName::clone(&frame_block.name),
                frame_block.symbol,
                match frame_block.location {
                    llir::Location::Global(val) => Some(val),
                    _ => None,
                },
            ));

            for run_block in &frame_block.runs {
                let mut code_block = CodeBlock::new(SymbolName::clone(&run_block.name), run_block.symbol, None);
                code_block.body = CodeGenerator::new(self.src_units, self.llir_blocks).generate(
                    &run_block.statements,
                )?;
                self.code_blocks.push(code_block);
            }
        }
        Ok(self.code_blocks)
    }
}

struct CodeGenerator<'a> {
    llir_blocks: &'a [llir::FrameBlock],
    registers: RegisterAllocator,
    code: Vec<Code>,
    src_units: &'a SrcUnits,
}

impl<'a> CodeGenerator<'a> {
    pub fn new<'b>(src_units: &'b SrcUnits, llir_blocks: &'b [llir::FrameBlock]) -> CodeGenerator<'b> {
        CodeGenerator {
            llir_blocks: llir_blocks,
            registers: RegisterAllocator::new(),
            code: Vec::new(),
            src_units: src_units,
        }
    }

    fn generate(mut self, llir_statements: &[llir::Statement]) -> error::Result<Vec<Code>> {
        let mut last_original_line = SrcTag::invalid();

        for statement in llir_statements {
            if statement.src_tag() != last_original_line {
                self.code.push(Code::Comment(
                    self.src_units.line_comment(statement.src_tag()),
                ));
                last_original_line = statement.src_tag();
            }
            self.code.push(Code::Comment(format!("{:?}", statement)));

            match *statement {
                llir::Statement::Add(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, carry| {
                        registers.add(body, param, carry)
                    })?;
                }
                llir::Statement::Subtract(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, carry| {
                        registers.subtract(body, param, carry)
                    })?;
                }
                llir::Statement::AddToDataStackPointer(ref data) => {
                    self.registers.load_dsp(&mut self.code, Register::Accum);
                    let add_param = Parameter::Immediate(match data.offset {
                        llir::SPOffset::Immediate(val) => val as u8,
                        llir::SPOffset::FrameSize(frame_ref) => self.lookup_frame_size(frame_ref)? as u8,
                        llir::SPOffset::NegativeFrameSize(frame_ref) => -self.lookup_frame_size(frame_ref)? as u8,
                    });
                    self.registers.add(&mut self.code, add_param, llir::CarryMode::ClearCarry);
                    self.registers.save_dsp_later(Register::Accum);
                    self.registers.load_dsp(&mut self.code, Register::XIndex);
                }
                llir::Statement::BranchIfZero(ref data) => {
                    self.registers.save_all_now(&mut self.code);
                    self.load_into_accum(&data.value)?;
                    self.code.push(Code::Beq(Parameter::Absolute(
                        Global::UnresolvedSymbol(data.destination),
                    )));
                }
                llir::Statement::CompareBranch(ref data) => {
                    let cmp_param = self.prepare_binary_op(&data.left, &data.right)?;
                    self.code.push(Code::Cmp(cmp_param));

                    if let Some(branch_set) = data.branch_set {
                        let param = Parameter::Absolute(Global::UnresolvedSymbol(branch_set));
                        self.code.push(match data.branch_flag {
                            llir::BranchFlag::Zero => Code::Beq(param),
                            llir::BranchFlag::Carry => Code::Bcs(param),
                        });
                    }

                    if let Some(branch_clear) = data.branch_clear {
                        let param = Parameter::Absolute(Global::UnresolvedSymbol(branch_clear));
                        self.code.push(match data.branch_flag {
                            llir::BranchFlag::Zero => Code::Bne(param),
                            llir::BranchFlag::Carry => Code::Bcc(param),
                        });
                    }
                }
                llir::Statement::CompareEq(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, _carry| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The Z flag in position 2 will be 1 if equal
                        body.push(Code::And(Parameter::Immediate(2)));
                        body.push(Code::Clc(Parameter::Implicit));
                        body.push(Code::Ror(Parameter::Implicit));
                    })?;
                }
                llir::Statement::CompareNotEq(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, _carry| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The Z flag in position 2 will be 1 if equal
                        // Use ExclusiveOR to negate it
                        body.push(Code::Eor(Parameter::Immediate(2)));
                        body.push(Code::And(Parameter::Immediate(2)));
                        body.push(Code::Clc(Parameter::Implicit));
                        body.push(Code::Ror(Parameter::Implicit));
                    })?;
                }
                llir::Statement::CompareLt(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, _carry| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The C flag in position 1 will be 1 if greater than or equal
                        // Use ExclusiveOR to negate it
                        body.push(Code::Eor(Parameter::Immediate(1)));
                        body.push(Code::And(Parameter::Immediate(1)));
                    })?;
                }
                llir::Statement::CompareGte(ref data) => {
                    self.generate_binary_op(data, |registers, body, param, _carry| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The C flag in position 1 will be 1 if greater than or equal
                        body.push(Code::And(Parameter::Immediate(1)));
                    })?;
                }
                llir::Statement::Copy(ref data) => {
                    self.load_into_accum(&data.value)?;
                    self.store_accum(&data.destination)?;
                }
                llir::Statement::GoTo(ref data) => {
                    self.registers.save_all_and_reset(&mut self.code);
                    self.code.push(Code::Jmp(Parameter::Absolute(
                        Global::UnresolvedSymbol(data.destination),
                    )));
                }
                llir::Statement::InlineAsm(ref data) => {
                    self.code.push(Code::InlineAsm(Arc::clone(&data.asm)));
                }
                llir::Statement::JumpRoutine(ref data) => {
                    self.registers.save_all_and_reset(&mut self.code);
                    self.code.push(Code::Jsr(Parameter::Absolute(match data.destination {
                        llir::Location::Global(addr) => Global::Resolved(addr),
                        llir::Location::UnresolvedGlobal(symbol) => Global::UnresolvedSymbol(symbol),
                        _ => unreachable!(),
                    })));
                }
                llir::Statement::Return(_) => {
                    self.registers.save_all_and_reset(&mut self.code);
                    self.code.push(Code::Rts(Parameter::Implicit));
                }
            }
        }
        self.registers.save_all_now(&mut self.code);
        Ok(self.code)
    }

    fn prepare_binary_op(&mut self, left: &llir::Value, right: &llir::Value) -> error::Result<Parameter> {
        // TODO: Choose left or right to go into accum based on least work
        self.load_into_accum(left)?;
        match *right {
            llir::Value::Immediate(ref _base_type, ref val) => Ok(Parameter::Immediate(val.number() as u8)),
            llir::Value::Memory(ref data) => {
                self.load_stack_pointer_if_necessary(&data.location)?;
                Ok(self.location_to_parameter(&data.location)?)
            }
        }
    }

    fn generate_binary_op<F>(&mut self, binary_op: &llir::BinaryOpData, code_gen: F) -> error::Result<()>
    where
        F: Fn(&mut RegisterAllocator,
           &mut Vec<Code>,
           Parameter,
           llir::CarryMode) -> (),
    {
        let param = self.prepare_binary_op(&binary_op.left, &binary_op.right)?;
        code_gen(
            &mut self.registers,
            &mut self.code,
            param,
            binary_op.carry_mode,
        );
        self.store_accum(&binary_op.destination)?;
        Ok(())
    }

    fn load_into_accum(&mut self, value: &llir::Value) -> error::Result<()> {
        self.load_value(Register::Accum, value)
    }

    pub fn load_value(&mut self, register: Register, value: &llir::Value) -> error::Result<()> {
        if let Register::XIndex = register {
            panic!(
                "The X register is required to load values as it's used \
                 for loading from the stack. Thus, you can't load a value directly into X."
            )
        }

        match *value {
            llir::Value::Immediate(ref _base_type, ref val) => {
                self.registers.load(
                    &mut self.code,
                    register,
                    Parameter::Immediate(val.number() as u8),
                )
            }
            llir::Value::Memory(ref data) => {
                match data.location {
                    llir::Location::Global(addr) => {
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::Absolute(Global::Resolved(addr)),
                        )
                    }
                    llir::Location::DataStackOffset(offset) => {
                        self.registers.load_dsp(&mut self.code, Register::XIndex);
                        self.registers.load(&mut self.code, register, Parameter::ZeroPageX(offset));
                    }
                    llir::Location::FrameOffset(frame_ref, offset) => {
                        self.registers.load_dsp(&mut self.code, Register::XIndex);
                        let frame_size = self.lookup_frame_size(frame_ref)?;
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::ZeroPageX(offset - frame_size),
                        );
                    }
                    llir::Location::FrameOffsetIndirect(frame_ref, offset) => {
                        self.registers.load_dsp(&mut self.code, Register::XIndex);
                        let dsp_offset = offset - self.lookup_frame_size(frame_ref)?;
                        self.registers.load(&mut self.code, register, Parameter::IndirectX(dsp_offset));
                    }
                    llir::Location::FrameOffsetBeforeCall(original_frame, calling_frame, offset) => {
                        let original_frame_size = self.lookup_frame_size(original_frame)?;
                        let call_to_frame_size = self.lookup_frame_size(calling_frame)?;
                        self.registers.load_dsp(&mut self.code, Register::XIndex);
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::ZeroPageX(offset - call_to_frame_size - original_frame_size),
                        );
                    }
                    llir::Location::GlobalIndexed(addr, ref index) => {
                        self.load_value(Register::YIndex, index)?;
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::AbsoluteY(Global::Resolved(addr)),
                        );
                    }
                    llir::Location::UnresolvedGlobal(symbol_ref) => {
                        let param = match data.base_type {
                            BaseType::U8 => Parameter::Absolute(Global::UnresolvedSymbol(symbol_ref)),
                            BaseType::U16 |
                            BaseType::Pointer(_) => Parameter::Absolute(Global::UnresolvedSymbolLowByte(symbol_ref)),
                            _ => unimplemented!(),
                        };
                        self.registers.load(&mut self.code, register, param);
                    }
                    llir::Location::UnresolvedGlobalLowByte(symbol_ref) => {
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::Absolute(Global::UnresolvedSymbolLowByte(symbol_ref)),
                        );
                    }
                    llir::Location::UnresolvedGlobalHighByte(symbol_ref) => {
                        self.registers.load(
                            &mut self.code,
                            register,
                            Parameter::Absolute(Global::UnresolvedSymbolHighByte(symbol_ref)),
                        );
                    }
                    _ => {
                        println!(
                            "WARN: Unimplemented load_value location: {:?}",
                            data.location
                        );
                        unimplemented!()
                    }
                }
            }
        }
        Ok(())
    }

    fn store_accum(&mut self, location: &llir::Location) -> error::Result<()> {
        self.load_stack_pointer_if_necessary(location)?;
        let param = self.location_to_parameter(location)?;
        self.registers.save_later(Register::Accum, param);
        Ok(())
    }

    fn lookup_frame_size(&self, symbol: SymbolRef) -> error::Result<i8> {
        for block in self.llir_blocks {
            if block.symbol == symbol {
                return Ok(block.frame_size);
            }
        }
        unreachable!("existence of frames should have been checked in previous stages")
    }

    fn load_stack_pointer_if_necessary(&mut self, location: &llir::Location) -> error::Result<()> {
        match *location {
            llir::Location::DataStackOffset(_) |
            llir::Location::FrameOffset(_, _) |
            llir::Location::FrameOffsetIndirect(_, _) => {
                self.registers.load_dsp(&mut self.code, Register::XIndex);
            }
            _ => {}
        }
        Ok(())
    }

    fn location_to_parameter(&mut self, location: &llir::Location) -> error::Result<Parameter> {
        match *location {
            llir::Location::Global(addr) => Ok(addr_param(addr)),
            llir::Location::GlobalIndexed(addr, ref index) => {
                self.load_value(Register::YIndex, index)?;
                Ok(Parameter::AbsoluteY(Global::Resolved(addr)))
            }
            llir::Location::DataStackOffset(offset) => Ok(Parameter::ZeroPageX(offset)),
            llir::Location::FrameOffset(frame_ref, offset) => Ok(Parameter::ZeroPageX(
                offset - self.lookup_frame_size(frame_ref)?,
            )),
            llir::Location::FrameOffsetIndirect(frame_ref, offset) => Ok(Parameter::IndirectX(
                offset - self.lookup_frame_size(frame_ref)?,
            )),
            llir::Location::UnresolvedGlobal(symbol) => Ok(Parameter::Absolute(Global::UnresolvedSymbol(symbol))),
            llir::Location::UnresolvedGlobalIndexed(symbol, ref index) => {
                self.load_value(Register::YIndex, index)?;
                Ok(Parameter::Absolute(Global::UnresolvedSymbol(symbol)))
            }
            _ => {
                println!("WARN: Unimplemented location_to_parameter: {:?}", location);
                unimplemented!()
            }
        }
    }
}

fn addr_param(addr: u16) -> Parameter {
    if addr < 256u16 {
        Parameter::ZeroPage(addr as u8)
    } else {
        Parameter::Absolute(Global::Resolved(addr))
    }
}
