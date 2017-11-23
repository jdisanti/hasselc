use code::{Code, Parameter};

pub const DATA_STACK_POINTER_LOCATION: u16 = 0x0000;

pub const DSP_PARAM: Parameter = Parameter::ZeroPage(DATA_STACK_POINTER_LOCATION as u8);
const DSP_REG_VALUE: RegisterValue = RegisterValue::Param(DSP_PARAM);

#[derive(Clone)]
pub struct RegisterEquivalency {
    equivalencies: Vec<RegisterValue>,
}

impl RegisterEquivalency {
    pub fn new() -> RegisterEquivalency {
        RegisterEquivalency {
            equivalencies: Vec::new(),
        }
    }

    pub fn clobber(&mut self, value: RegisterValue) {
        self.equivalencies.clear();
        self.equivalencies.push(value);
    }

    pub fn add_value(&mut self, value: RegisterValue) {
        if !self.is_equivalent(&value) {
            self.equivalencies.push(value);
        }
    }

    pub fn is_equivalent(&self, value: &RegisterValue) -> bool {
        self.equivalencies.contains(value)
    }

    pub fn reset(&mut self) {
        self.equivalencies.clear();
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegisterValue {
    Intermediate(usize),
    Param(Parameter),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Register {
    Accum,
    XIndex,
    YIndex,
}

impl Register {
    pub fn ordinal(&self) -> usize {
        match *self {
            Register::Accum => 0,
            Register::XIndex => 1,
            Register::YIndex => 2,
        }
    }

    pub fn from_ordinal(ordinal: usize) -> Register {
        match ordinal {
            0 => Register::Accum,
            1 => Register::XIndex,
            2 => Register::YIndex,
            _ => panic!("Invalid Register ordinal: {}", ordinal),
        }
    }

    pub fn load_op(&self, parameter: Parameter) -> Code {
        match *self {
            Register::Accum => Code::Lda(parameter),
            Register::XIndex => Code::Ldx(parameter),
            Register::YIndex => Code::Ldy(parameter),
        }
    }

    pub fn save_op(&self, parameter: Parameter) -> Code {
        match *self {
            Register::Accum => Code::Sta(parameter),
            Register::XIndex => Code::Stx(parameter),
            Register::YIndex => Code::Sty(parameter),
        }
    }

    pub fn to_other(&self, register: Register) -> Option<Code> {
        use self::Register::*;
        match *self {
            Accum => match register {
                Accum => None,
                XIndex => Some(Code::Tax(Parameter::Implicit)),
                YIndex => Some(Code::Tay(Parameter::Implicit)),
            },
            XIndex => match register {
                Accum => Some(Code::Txa(Parameter::Implicit)),
                XIndex | YIndex => None,
            },
            YIndex => match register {
                Accum => Some(Code::Tya(Parameter::Implicit)),
                XIndex | YIndex => None,
            },
        }
    }
}

struct SaveLocation(pub Parameter);

impl SaveLocation {
    pub fn requires(&self, register: Register) -> bool {
        match self.0 {
            Parameter::AbsoluteX(_) | Parameter::IndirectX(_) | Parameter::ZeroPageX(_) => register == Register::XIndex,
            Parameter::AbsoluteY(_) | Parameter::IndirectY(_) | Parameter::ZeroPageY(_) => register == Register::YIndex,
            _ => false,
        }
    }
}

pub struct RegisterAllocator {
    values: [RegisterEquivalency; 3],
    save_locations: [Vec<SaveLocation>; 3],
    next_intermediate_differentiator: usize,
}

impl RegisterAllocator {
    pub fn new() -> RegisterAllocator {
        RegisterAllocator {
            values: [
                RegisterEquivalency::new(),
                RegisterEquivalency::new(),
                RegisterEquivalency::new(),
            ],
            save_locations: [Vec::new(), Vec::new(), Vec::new()],
            next_intermediate_differentiator: 0,
        }
    }

    fn next_intermediate(&mut self) -> RegisterValue {
        let result = RegisterValue::Intermediate(self.next_intermediate_differentiator);
        self.next_intermediate_differentiator += 1;
        result
    }

    fn spillover(&mut self, code: &mut Vec<Code>, register: Register) {
        while let Some(location) = self.save_locations[register.ordinal()].pop() {
            code.push(register.save_op(location.0.clone()));
            self.values[register.ordinal()].add_value(RegisterValue::Param(location.0));
        }
    }

    fn save_as_necessary(&mut self, code: &mut Vec<Code>, clobbering: Register) {
        let mut spillovers = [false; 3];
        spillovers[clobbering.ordinal()] = true;
        for (index, locations) in self.save_locations.iter().enumerate() {
            for location in locations {
                if location.requires(clobbering) {
                    spillovers[index] = true;
                }
            }
        }

        for (index, spillover) in spillovers.iter().enumerate() {
            if *spillover {
                self.spillover(code, Register::from_ordinal(index));
            }
        }
    }

    pub fn add(&mut self, code: &mut Vec<Code>, param: Parameter) {
        self.save_as_necessary(code, Register::Accum);
        code.push(Code::Clc(Parameter::Implicit));
        code.push(Code::Adc(param));
        let next_intermediate = self.next_intermediate();
        self.values[Register::Accum.ordinal()].clobber(next_intermediate);
    }

    pub fn subtract(&mut self, code: &mut Vec<Code>, param: Parameter) {
        self.save_as_necessary(code, Register::Accum);
        code.push(Code::Sec(Parameter::Implicit));
        code.push(Code::Sbc(param));
        let next_intermediate = self.next_intermediate();
        self.values[Register::Accum.ordinal()].clobber(next_intermediate);
    }

    pub fn load_status_into_accum(&mut self, code: &mut Vec<Code>) {
        self.save_as_necessary(code, Register::Accum);
        code.push(Code::Php(Parameter::Implicit));
        code.push(Code::Pla(Parameter::Implicit));
        let next_intermediate = self.next_intermediate();
        self.values[Register::Accum.ordinal()].clobber(next_intermediate);
    }

    pub fn load(&mut self, code: &mut Vec<Code>, register: Register, param: Parameter) {
        if !self.values[register.ordinal()].is_equivalent(&RegisterValue::Param(param.clone())) {
            self.save_as_necessary(code, register);
            code.push(register.load_op(param.clone()));
            self.values[register.ordinal()].clobber(RegisterValue::Param(param));
        }
    }

    pub fn save_later(&mut self, register: Register, location: Parameter) {
        self.save_locations[register.ordinal()].push(SaveLocation(location.clone()));
        self.values[register.ordinal()].add_value(RegisterValue::Param(location));
    }

    pub fn save_all_now(&mut self, code: &mut Vec<Code>) {
        self.spillover(code, Register::Accum);
        self.spillover(code, Register::XIndex);
        self.spillover(code, Register::YIndex);
    }

    pub fn save_all_and_reset(&mut self, code: &mut Vec<Code>) {
        self.save_all_now(code);
        for mut value in &mut self.values {
            value.reset();
        }
        self.save_locations = [Vec::new(), Vec::new(), Vec::new()];
    }

    pub fn load_dsp(&mut self, code: &mut Vec<Code>, into: Register) {
        // If we already have the stack pointer loaded somewhere, just re-use it
        for i in 0..self.values.len() {
            if self.values[i].is_equivalent(&DSP_REG_VALUE) {
                if Register::from_ordinal(i) == into {
                    return;
                } else if let Some(transfer) = Register::from_ordinal(i).to_other(into) {
                    self.save_as_necessary(code, into);
                    code.push(transfer);
                    return;
                }
            }
        }

        // Otherwise, load it
        self.load(code, into, DSP_PARAM);
    }

    pub fn save_dsp_later(&mut self, from_register: Register) {
        self.save_later(from_register, DSP_PARAM);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use code::CodeBlock;

    #[test]
    fn store_then_load_equivalency() {
        let mut code_block = CodeBlock::new(None, None);
        let mut registers = RegisterAllocator::new();

        registers.load(
            &mut code_block.body,
            Register::Accum,
            Parameter::ZeroPage(5),
        );
        registers.add(&mut code_block.body, Parameter::ZeroPage(6));
        registers.save_later(Register::Accum, Parameter::ZeroPage(5));
        registers.load(
            &mut code_block.body,
            Register::Accum,
            Parameter::ZeroPage(5),
        );

        assert_eq!(
            "\
             \tLDA\t$05\n\
             \tCLC\t\n\
             \tADC\t$06\n",
            code_block.to_asm().unwrap()
        );
    }

    #[test]
    fn save_as_necessary_a_only() {
        let mut code_block = CodeBlock::new(None, None);
        let mut registers = RegisterAllocator::new();

        registers.load(
            &mut code_block.body,
            Register::Accum,
            Parameter::Immediate(0),
        );
        registers.save_later(Register::Accum, Parameter::ZeroPage(0));
        registers.load(
            &mut code_block.body,
            Register::Accum,
            Parameter::Immediate(1),
        );

        assert_eq!(
            "\
             \tLDA\t#0\n\
             \tSTA\t$00\n\
             \tLDA\t#1\n",
            code_block.to_asm().unwrap()
        );
    }

    #[test]
    fn save_as_necessary_x_change() {
        let mut code_block = CodeBlock::new(None, None);
        let mut registers = RegisterAllocator::new();

        registers.load(
            &mut code_block.body,
            Register::XIndex,
            Parameter::Immediate(0),
        );
        registers.load(
            &mut code_block.body,
            Register::Accum,
            Parameter::Immediate(5),
        );
        registers.save_later(Register::Accum, Parameter::ZeroPageX(2));
        registers.load(
            &mut code_block.body,
            Register::XIndex,
            Parameter::Immediate(1),
        );

        assert_eq!(
            "\
             \tLDX\t#0\n\
             \tLDA\t#5\n\
             \tSTA\t$02, X\n\
             \tLDX\t#1\n",
            code_block.to_asm().unwrap()
        );
    }
}
