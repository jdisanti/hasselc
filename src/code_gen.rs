use std::sync::Arc;
use code::{Code, CodeBlock, Global, Parameter};
use error;
use llir;
use register::{Register, RegisterAllocator};
use symbol_table::SymbolRef;

pub struct CodeBlockGenerator<'a> {
    llir_blocks: &'a [llir::FrameBlock],
    code_blocks: Vec<CodeBlock>,
}

impl<'a> CodeBlockGenerator<'a> {
    pub fn new(input: &[llir::FrameBlock]) -> CodeBlockGenerator {
        CodeBlockGenerator {
            llir_blocks: input,
            code_blocks: Vec::new(),
        }
    }

    pub fn generate(mut self) -> error::Result<Vec<CodeBlock>> {
        for frame_block in self.llir_blocks {
            self.code_blocks.push(CodeBlock::new(
                frame_block.name.clone(),
                match frame_block.location {
                    llir::Location::Global(val) => Some(val),
                    _ => None,
                },
            ));

            for run_block in &frame_block.runs {
                let mut code_block = CodeBlock::new(Some(Arc::clone(&run_block.name)), None);
                code_block.body = CodeGenerator::new(self.llir_blocks).generate(&run_block.statements)?;
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
}

impl<'a> CodeGenerator<'a> {
    pub fn new(llir_blocks: &[llir::FrameBlock]) -> CodeGenerator {
        CodeGenerator {
            llir_blocks: llir_blocks,
            registers: RegisterAllocator::new(),
            code: Vec::new(),
        }
    }

    fn generate(mut self, llir_statements: &[llir::Statement]) -> error::Result<Vec<Code>> {
        for statement in llir_statements {
            match *statement {
                llir::Statement::Add(ref data) => {
                    self.generate_binary_op(data, |registers, body, param| registers.add(body, param))?;
                }
                llir::Statement::Subtract(ref data) => {
                    self.generate_binary_op(
                        data,
                        |registers, body, param| registers.subtract(body, param),
                    )?;
                }
                llir::Statement::AddToDataStackPointer(ref val) => {
                    self.registers.load_dsp(&mut self.code, Register::Accum);
                    let add_param = Parameter::Immediate(match *val {
                        llir::SPOffset::Immediate(val) => val as u8,
                        llir::SPOffset::FrameSize(ref name) => self.lookup_frame_size(name)? as u8,
                        llir::SPOffset::NegativeFrameSize(ref name) => -self.lookup_frame_size(name)? as u8,
                    });
                    self.registers.add(&mut self.code, add_param);
                    self.registers.save_dsp_later(Register::Accum);
                    self.registers.load_dsp(&mut self.code, Register::XIndex);
                }
                llir::Statement::BranchIfZero(ref data) => {
                    self.registers.save_all_now(&mut self.code);
                    self.load_into_accum(&data.value)?;
                    self.code.push(Code::Beq(Parameter::Absolute(
                        Global::UnresolvedName(SymbolRef::clone(&data.destination)),
                    )));
                }
                llir::Statement::CompareEq(ref data) => {
                    self.generate_binary_op(data, |registers, body, param| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The Z flag in position 2 will be 1 if equal
                        body.push(Code::And(Parameter::Immediate(2)));
                        body.push(Code::Clc(Parameter::Implicit));
                        body.push(Code::Ror(Parameter::Implicit));
                    })?;
                }
                llir::Statement::CompareNotEq(ref data) => {
                    self.generate_binary_op(data, |registers, body, param| {
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
                    self.generate_binary_op(data, |registers, body, param| {
                        body.push(Code::Cmp(param));
                        registers.load_status_into_accum(body);
                        // The C flag in position 1 will be 1 if greater than or equal
                        // Use ExclusiveOR to negate it
                        body.push(Code::Eor(Parameter::Immediate(1)));
                        body.push(Code::And(Parameter::Immediate(1)));
                    })?;
                }
                llir::Statement::CompareGte(ref data) => {
                    self.generate_binary_op(data, |registers, body, param| {
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
                        Global::UnresolvedName(SymbolRef::clone(&data.destination)),
                    )));
                }
                llir::Statement::JumpRoutine(ref data) => {
                    self.registers.save_all_and_reset(&mut self.code);
                    self.code
                        .push(Code::Jsr(Parameter::Absolute(match data.destination {
                            llir::Location::Global(addr) => Global::Resolved(addr),
                            llir::Location::UnresolvedGlobal(ref name) => {
                                Global::UnresolvedName(SymbolRef::clone(name))
                            }
                            _ => unreachable!(),
                        })));
                }
                llir::Statement::Return => {
                    self.registers.save_all_and_reset(&mut self.code);
                    self.code.push(Code::Rts(Parameter::Implicit));
                }
            }
        }
        self.registers.save_all_now(&mut self.code);
        Ok(self.code)
    }

    fn generate_binary_op<F>(&mut self, binary_op: &llir::BinaryOpData, code_gen: F) -> error::Result<()>
    where
        F: Fn(&mut RegisterAllocator, &mut Vec<Code>, Parameter) -> (),
    {
        // TODO: Choose left or right to go into accum based on least work
        self.load_into_accum(&binary_op.left)?;
        match binary_op.right {
            llir::Value::Immediate(val) => {
                code_gen(
                    &mut self.registers,
                    &mut self.code,
                    Parameter::Immediate(val),
                );
            }
            llir::Value::Memory(ref location) => {
                self.load_stack_pointer_if_necessary(location)?;
                let param = self.location_to_parameter(location)?;
                code_gen(&mut self.registers, &mut self.code, param);
            }
        }
        self.store_accum(&binary_op.destination)?;
        Ok(())
    }

    fn load_into_accum(&mut self, value: &llir::Value) -> error::Result<()> {
        match *value {
            llir::Value::Immediate(val) => {
                self.registers
                    .load(&mut self.code, Register::Accum, Parameter::Immediate(val));
            }
            llir::Value::Memory(ref location) => match *location {
                llir::Location::Global(addr) => {
                    self.registers
                        .load(&mut self.code, Register::Accum, addr_param(addr));
                }
                llir::Location::DataStackOffset(offset) => {
                    self.registers.load_dsp(&mut self.code, Register::XIndex);
                    self.registers.load(
                        &mut self.code,
                        Register::Accum,
                        Parameter::ZeroPageX(offset),
                    );
                }
                llir::Location::FrameOffset(ref frame, offset) => {
                    self.registers.load_dsp(&mut self.code, Register::XIndex);
                    let frame_size = self.lookup_frame_size(frame)?;
                    self.registers.load(
                        &mut self.code,
                        Register::Accum,
                        Parameter::ZeroPageX(offset - frame_size),
                    );
                }
                llir::Location::FrameOffsetBeforeCall(ref original_frame, ref calling_frame, offset) => {
                    let original_frame_size = self.lookup_frame_size(original_frame)?;
                    let call_to_frame_size = self.lookup_frame_size(calling_frame)?;
                    self.registers.load_dsp(&mut self.code, Register::XIndex);
                    self.registers.load(
                        &mut self.code,
                        Register::Accum,
                        Parameter::ZeroPageX(offset - call_to_frame_size - original_frame_size),
                    );
                }
                _ => {
                    println!(
                        "WARN: Unimplemented load_into_accum location: {:?}",
                        location
                    );
                }
            },
        }
        Ok(())
    }

    fn store_accum(&mut self, location: &llir::Location) -> error::Result<()> {
        self.load_stack_pointer_if_necessary(location)?;
        let param = self.location_to_parameter(location)?;
        self.registers.save_later(Register::Accum, param);
        Ok(())
    }

    fn lookup_frame_size(&self, name: &SymbolRef) -> error::Result<i8> {
        for block in self.llir_blocks {
            if block.name.is_some() && block.name.as_ref().unwrap() == name {
                return Ok(block.frame_size);
            }
        }
        // TODO: Error: not found
        unimplemented!()
    }

    fn load_stack_pointer_if_necessary(&mut self, location: &llir::Location) -> error::Result<()> {
        match *location {
            llir::Location::DataStackOffset(_) | llir::Location::FrameOffset(_, _) => {
                self.registers.load_dsp(&mut self.code, Register::XIndex);
            }
            _ => {}
        }
        Ok(())
    }

    fn location_to_parameter(&self, location: &llir::Location) -> error::Result<Parameter> {
        match *location {
            llir::Location::Global(addr) => Ok(addr_param(addr)),
            llir::Location::DataStackOffset(offset) => Ok(Parameter::ZeroPageX(offset)),
            llir::Location::FrameOffset(ref frame, offset) => Ok(Parameter::ZeroPageX(
                offset - self.lookup_frame_size(frame)?,
            )),
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
