use code::{Code, Parameter};
use std::mem;

pub const DATA_STACK_POINTER_LOCATION: u16 = 0x0000;

pub const DSP_PARAM: Parameter = Parameter::ZeroPage(DATA_STACK_POINTER_LOCATION as u8);
const DSP_REG_VALUE: RegisterValue = RegisterValue::Param(DSP_PARAM);

#[derive(Debug, Eq, PartialEq)]
pub enum RegisterValue {
    Uninitialized,
    Intermediate(usize),
    Param(Parameter),
}

impl RegisterValue {
    pub fn param(&self) -> Option<&Parameter> {
        match *self {
            RegisterValue::Param(ref param) => Some(param),
            _ => None,
        }
    }
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
                XIndex => None,
                YIndex => None,
            },
            YIndex => match register {
                Accum => Some(Code::Tya(Parameter::Implicit)),
                XIndex => None,
                YIndex => None,
            },
        }
    }
}

struct SaveLocation {
    pub expect_x: Option<Parameter>,
    pub expect_y: Option<Parameter>,
    pub location: Parameter,
}

impl SaveLocation {
    pub fn new(expect_x: Option<Parameter>, expect_y: Option<Parameter>, location: Parameter) -> SaveLocation {
        SaveLocation {
            expect_x: expect_x,
            expect_y: expect_y,
            location: location,
        }
    }
}

pub struct RegisterAllocator {
    values: [RegisterValue; 3],
    save_locations: [Option<SaveLocation>; 3],
    next_intermediate_differentiator: usize,
}

impl RegisterAllocator {
    pub fn new() -> RegisterAllocator {
        RegisterAllocator {
            values: [
                RegisterValue::Uninitialized,
                RegisterValue::Uninitialized,
                RegisterValue::Uninitialized,
            ],
            save_locations: [None, None, None],
            next_intermediate_differentiator: 0,
        }
    }

    fn next_intermediate(&mut self) -> RegisterValue {
        let result = RegisterValue::Intermediate(self.next_intermediate_differentiator);
        self.next_intermediate_differentiator += 1;
        result
    }

    fn spillover(&mut self, code: &mut Vec<Code>, register: Register) {
        let mut save_location = None;
        mem::swap(
            &mut save_location,
            &mut self.save_locations[register.ordinal()],
        );
        if let Some(location) = save_location {
            if register != Register::XIndex {
                if location.expect_x.as_ref() != self.values[Register::XIndex.ordinal()].param() {
                    if let Some(expected_x) = location.expect_x {
                        self.spillover(code, Register::XIndex);
                        self.load(code, Register::XIndex, expected_x.clone());
                    }
                }
            }
            if register != Register::YIndex {
                if location.expect_y.as_ref() != self.values[Register::YIndex.ordinal()].param() {
                    if let Some(expected_y) = location.expect_y {
                        self.spillover(code, Register::YIndex);
                        self.load(code, Register::YIndex, expected_y.clone());
                    }
                }
            }
            code.push(register.save_op(location.location.clone()));
            self.values[register.ordinal()] = RegisterValue::Param(location.location);
        }
    }

    pub fn add(&mut self, code: &mut Vec<Code>, param: Parameter) {
        self.spillover(code, Register::Accum);
        code.push(Code::Adc(param));
        self.values[Register::Accum.ordinal()] = self.next_intermediate();
    }

    pub fn load(&mut self, code: &mut Vec<Code>, register: Register, param: Parameter) {
        if self.values[register.ordinal()] != RegisterValue::Param(param.clone()) {
            self.spillover(code, register);
            code.push(register.load_op(param.clone()));
            self.values[register.ordinal()] = RegisterValue::Param(param);
        }
    }

    pub fn save_later(
        &mut self,
        register: Register,
        expect_x: Option<Parameter>,
        expect_y: Option<Parameter>,
        location: Parameter,
    ) {
        self.save_locations[register.ordinal()] = Some(SaveLocation::new(expect_x, expect_y, location));
    }

    pub fn save_all_now(&mut self, code: &mut Vec<Code>) {
        self.spillover(code, Register::Accum);
        self.spillover(code, Register::XIndex);
        self.spillover(code, Register::YIndex);
    }

    pub fn save_all_and_reset(&mut self, code: &mut Vec<Code>) {
        self.save_all_now(code);
        self.values = [
            RegisterValue::Uninitialized,
            RegisterValue::Uninitialized,
            RegisterValue::Uninitialized,
        ];
        self.save_locations = [None, None, None];
    }

    pub fn load_dsp(&mut self, code: &mut Vec<Code>, into: Register) {
        // If we already have the stack pointer loaded somewhere, just re-use it
        for i in 0..self.values.len() {
            if self.values[i] == DSP_REG_VALUE {
                if Register::from_ordinal(i) == into {
                    return;
                } else if let Some(transfer) = Register::from_ordinal(i).to_other(into) {
                    self.spillover(code, into);
                    code.push(transfer);
                    return;
                }
            }
        }

        // Otherwise, load it
        self.load(code, into, DSP_PARAM);
    }

    pub fn save_dsp_later(&mut self, from_register: Register) {
        self.save_later(from_register, None, None, DSP_PARAM);
    }
}
