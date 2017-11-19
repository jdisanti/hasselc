use std::fmt::Write;
use std::sync::Arc;
use error;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Global {
    Resolved(u16),
    UnresolvedBlock,
    UnresolvedName(Arc<String>),
}

impl Global {
    fn to_asm(&self) -> String {
        match *self {
            Global::Resolved(val) => format!("${:04X}", val),
            Global::UnresolvedBlock => format!("UNRESOLVED_BLOCK"),
            Global::UnresolvedName(ref name) => format!("{}", name),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Parameter {
    Implicit,
    Accumulator,
    Immediate(u8),
    ZeroPage(u8),
    ZeroPageX(i8),
    ZeroPageY(i8),
    Relative(i8),
    Absolute(Global),
    AbsoluteX(Global),
    AbsoluteY(Global),
    Indirect(Global),
    IndirectX(i8),
    IndirectY(i8),
}

impl Parameter {
    fn to_asm(&self) -> String {
        match *self {
            Parameter::Implicit => String::from(""),
            Parameter::Accumulator => format!("A"),
            Parameter::Immediate(val) => format!("#{}", val),
            Parameter::ZeroPage(offset) => format!("${:02X}", offset),
            Parameter::ZeroPageX(offset) => format!("${:02X}, X", offset),
            Parameter::ZeroPageY(offset) => format!("${:02X}, Y", offset),
            Parameter::Absolute(ref gbl) => gbl.to_asm(),
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Code {
    Adc(Parameter),
    Clc(Parameter),
    Jmp(Parameter),
    Jsr(Parameter),
    Lda(Parameter),
    Ldx(Parameter),
    Ldy(Parameter),
    Rts(Parameter),
    Sbc(Parameter),
    Sec(Parameter),
    Sta(Parameter),
    Stx(Parameter),
    Sty(Parameter),
    Tax(Parameter),
    Tay(Parameter),
    Txa(Parameter),
    Tya(Parameter),
}

impl Code {
    pub fn parameter(&self) -> &Parameter {
        match *self {
            Code::Adc(ref p) => p,
            Code::Clc(ref p) => p,
            Code::Jmp(ref p) => p,
            Code::Jsr(ref p) => p,
            Code::Lda(ref p) => p,
            Code::Ldx(ref p) => p,
            Code::Ldy(ref p) => p,
            Code::Rts(ref p) => p,
            Code::Sbc(ref p) => p,
            Code::Sec(ref p) => p,
            Code::Sta(ref p) => p,
            Code::Stx(ref p) => p,
            Code::Sty(ref p) => p,
            Code::Tax(ref p) => p,
            Code::Tay(ref p) => p,
            Code::Txa(ref p) => p,
            Code::Tya(ref p) => p,
        }
    }

    pub fn is_branch(&self) -> bool {
        match *self {
            Code::Jsr(_) | Code::Rts(_) | Code::Jmp(_) => true,
            _ => false,
        }
    }

    pub fn to_asm(&self) -> String {
        match *self {
            Code::Adc(ref p) => format!("ADC\t{}", p.to_asm()),
            Code::Clc(ref p) => format!("CLC\t{}", p.to_asm()),
            Code::Jmp(ref p) => format!("JMP\t{}", p.to_asm()),
            Code::Jsr(ref p) => format!("JSR\t{}", p.to_asm()),
            Code::Lda(ref p) => format!("LDA\t{}", p.to_asm()),
            Code::Ldx(ref p) => format!("LDX\t{}", p.to_asm()),
            Code::Ldy(ref p) => format!("LDY\t{}", p.to_asm()),
            Code::Rts(ref p) => format!("RTS\t{}", p.to_asm()),
            Code::Sbc(ref p) => format!("SBC\t{}", p.to_asm()),
            Code::Sec(ref p) => format!("SEC\t{}", p.to_asm()),
            Code::Sta(ref p) => format!("STA\t{}", p.to_asm()),
            Code::Stx(ref p) => format!("STX\t{}", p.to_asm()),
            Code::Sty(ref p) => format!("STY\t{}", p.to_asm()),
            Code::Tax(ref p) => format!("TAX\t{}", p.to_asm()),
            Code::Tay(ref p) => format!("TAY\t{}", p.to_asm()),
            Code::Txa(ref p) => format!("TXA\t{}", p.to_asm()),
            Code::Tya(ref p) => format!("TYA\t{}", p.to_asm()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CodeBlock {
    pub location: Global,
    pub name: Option<Arc<String>>,
    pub body: Vec<Code>,
}

impl CodeBlock {
    pub fn new(name: Option<Arc<String>>, location: Option<u16>) -> CodeBlock {
        CodeBlock {
            location: match location {
                Some(val) => Global::Resolved(val),
                None => Global::UnresolvedBlock,
            },
            name: name,
            body: Vec::new(),
        }
    }

    pub fn to_asm(&self) -> error::Result<String> {
        let mut asm = String::new();
        if let Global::Resolved(addr) = self.location {
            write!(asm, ".org ${:04X}\n\n", addr)?;
        }
        if let Some(ref name) = self.name {
            write!(asm, "\n{}:\n", name)?;
        }
        for code in &self.body {
            write!(asm, "\t{}\n", code.to_asm())?;
        }
        Ok(asm)
    }
}
