use std::sync::Arc;
use llir;
use code;
use error;
use register::{Register, RegisterAllocator};

const RETURN_LOCATION_LO: u16 = 0x0001;
const RETURN_LOCATION_HI: u16 = 0x0002;

pub fn generate_code(input: &Vec<llir::Block>) -> error::Result<Vec<code::CodeBlock>> {
    let mut code_blocks = Vec::new();
    for llir_block in input {
        let mut code_block = code::CodeBlock::new(
            llir_block.name.clone(),
            match llir_block.location {
                llir::Location::Global(val) => Some(val),
                _ => None,
            },
        );
        code_block.body = generate_body(input, &llir_block.statements)?;
        code_blocks.push(code_block);
    }
    Ok(code_blocks)
}

fn generate_body(blocks: &Vec<llir::Block>, input: &Vec<llir::Statement>) -> error::Result<Vec<code::Code>> {
    use code::{Code, Global, Parameter};

    let mut registers = RegisterAllocator::new();

    let mut body = Vec::new();
    for statement in input {
        match *statement {
            llir::Statement::AddToDataStackPointer(ref val) => {
                registers.load_dsp(&mut body, Register::Accum);
                registers.add(
                    &mut body,
                    Parameter::Immediate(match *val {
                        llir::SPOffset::Immediate(val) => val as u8,
                        llir::SPOffset::FrameSize(ref name) => lookup_frame_size(blocks, name.clone())? as u8,
                        llir::SPOffset::NegativeFrameSize(ref name) => -lookup_frame_size(blocks, name.clone())? as u8,
                    }),
                );
                registers.save_dsp_later(Register::Accum);
                registers.load_dsp(&mut body, Register::XIndex);
            }
            llir::Statement::Copy(ref data) => {
                generate_store(
                    &mut registers,
                    &mut body,
                    blocks,
                    &data.destination,
                    &data.value,
                )?;
            }
            llir::Statement::JumpRoutine(ref location) => {
                registers.save_all_and_reset(&mut body);
                body.push(Code::Jsr(Parameter::Absolute(match *location {
                    llir::Location::Global(addr) => Global::Resolved(addr),
                    llir::Location::UnresolvedGlobal(ref name) => Global::UnresolvedName(name.clone()),
                    _ => unreachable!(),
                })));
            }
            llir::Statement::Return => {
                registers.save_all_and_reset(&mut body);
                body.push(Code::Rts(Parameter::Implicit));
            }
            llir::Statement::Add(ref data) => {
                generate_add(
                    &mut registers,
                    &mut body,
                    blocks,
                    &data.destination,
                    &data.left,
                    &data.right,
                )?;
            }
            llir::Statement::GoTo(ref name) => {
                registers.save_all_and_reset(&mut body);
                body.push(Code::Jmp(
                    Parameter::Absolute(Global::UnresolvedName(name.clone())),
                ));
            }
            _ => {
                println!("WARN: Unimplemented generate_body: {:?}", statement);
            }
        }
    }
    Ok(body)
}

fn lookup_frame_size(blocks: &Vec<llir::Block>, name: Arc<String>) -> error::Result<i8> {
    for block in blocks {
        if block.name.is_some() && block.name.as_ref().unwrap() == &name {
            return Ok(block.frame_size);
        }
    }
    // TODO: Error: not found
    unimplemented!()
}

fn generate_store(
    registers: &mut RegisterAllocator,
    body: &mut Vec<code::Code>,
    blocks: &Vec<llir::Block>,
    dest: &llir::Location,
    value: &llir::Value,
) -> error::Result<()> {
    load_into_accum(registers, body, blocks, value)?;
    store_accum(registers, body, blocks, dest)?;
    Ok(())
}

fn generate_add(
    registers: &mut RegisterAllocator,
    body: &mut Vec<code::Code>,
    blocks: &Vec<llir::Block>,
    dest: &llir::Location,
    left: &llir::Value,
    right: &llir::Value,
) -> error::Result<()> {
    use code::Parameter;
    load_into_accum(registers, body, blocks, left)?;
    match *right {
        llir::Value::Immediate(val) => {
            registers.add(body, Parameter::Immediate(val));
        }
        llir::Value::Memory(ref location) => {
            load_stack_pointer_if_necessary(registers, body, location)?;
            registers.add(body, location_to_parameter(blocks, location)?);
        }
    }
    store_accum(registers, body, blocks, dest)?;
    Ok(())
}

fn load_stack_pointer_if_necessary(
    registers: &mut RegisterAllocator,
    body: &mut Vec<code::Code>,
    location: &llir::Location,
) -> error::Result<()> {
    match *location {
        llir::Location::DataStackOffset(_) | llir::Location::FrameOffset(_, _) => {
            registers.load_dsp(body, Register::XIndex);
        }
        _ => {}
    }
    Ok(())
}

fn location_to_parameter(blocks: &Vec<llir::Block>, location: &llir::Location) -> error::Result<code::Parameter> {
    use code::Parameter;
    match *location {
        llir::Location::Global(addr) => Ok(addr_param(addr)),
        llir::Location::DataStackOffset(offset) => Ok(Parameter::ZeroPageX(offset)),
        llir::Location::FrameOffset(ref frame, offset) => Ok(Parameter::ZeroPageX(
            offset - lookup_frame_size(blocks, frame.clone())?,
        )),
        _ => {
            println!("WARN: Unimplemented location_to_parameter: {:?}", location);
            unimplemented!()
        }
    }
}

fn store_accum(
    registers: &mut RegisterAllocator,
    body: &mut Vec<code::Code>,
    blocks: &Vec<llir::Block>,
    location: &llir::Location,
) -> error::Result<()> {
    load_stack_pointer_if_necessary(registers, body, location)?;
    registers.save_later(Register::Accum, location_to_parameter(blocks, location)?);
    Ok(())
}

fn load_into_accum(
    registers: &mut RegisterAllocator,
    body: &mut Vec<code::Code>,
    blocks: &Vec<llir::Block>,
    value: &llir::Value,
) -> error::Result<()> {
    use code::Parameter;
    match *value {
        llir::Value::Immediate(val) => {
            registers.load(body, Register::Accum, Parameter::Immediate(val));
        }
        llir::Value::Memory(ref location) => match *location {
            llir::Location::Global(addr) => {
                registers.load(body, Register::Accum, addr_param(addr));
            }
            llir::Location::DataStackOffset(offset) => {
                registers.load_dsp(body, Register::XIndex);
                registers.load(body, Register::Accum, Parameter::ZeroPageX(offset));
            }
            llir::Location::FrameOffset(ref frame, offset) => {
                registers.load_dsp(body, Register::XIndex);
                registers.load(
                    body,
                    Register::Accum,
                    Parameter::ZeroPageX(offset - lookup_frame_size(blocks, frame.clone())?),
                );
            }
            llir::Location::FrameOffsetBeforeCall(ref original_frame, ref calling_frame, offset) => {
                let original_frame_size = lookup_frame_size(blocks, original_frame.clone())?;
                let call_to_frame_size = lookup_frame_size(blocks, calling_frame.clone())?;
                registers.load_dsp(body, Register::XIndex);
                registers.load(
                    body,
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

fn addr_param(addr: u16) -> code::Parameter {
    use code::{Global, Parameter};
    if addr < 256u16 {
        Parameter::ZeroPage(addr as u8)
    } else {
        Parameter::Absolute(Global::Resolved(addr))
    }
}
