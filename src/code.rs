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
    Lda(Parameter),
    Sta(Parameter),
    Ldx(Parameter),
    Stx(Parameter),
    Sty(Parameter),
    Tax(Parameter),
    Txa(Parameter),
    Sbc(Parameter),
    Clc(Parameter),
    Sec(Parameter),
    Jsr(Parameter),
    Jmp(Parameter),
    Rts(Parameter),
}

impl Code {
    pub fn parameter(&self) -> &Parameter {
        match *self {
            Code::Adc(ref p) => p,
            Code::Lda(ref p) => p,
            Code::Sta(ref p) => p,
            Code::Ldx(ref p) => p,
            Code::Stx(ref p) => p,
            Code::Sty(ref p) => p,
            Code::Tax(ref p) => p,
            Code::Txa(ref p) => p,
            Code::Sbc(ref p) => p,
            Code::Clc(ref p) => p,
            Code::Sec(ref p) => p,
            Code::Jsr(ref p) => p,
            Code::Jmp(ref p) => p,
            Code::Rts(ref p) => p,
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
            Code::Adc(ref p) => format!("ADC    {}", p.to_asm()),
            Code::Lda(ref p) => format!("LDA    {}", p.to_asm()),
            Code::Sta(ref p) => format!("STA    {}", p.to_asm()),
            Code::Ldx(ref p) => format!("LDX    {}", p.to_asm()),
            Code::Stx(ref p) => format!("STX    {}", p.to_asm()),
            Code::Sty(ref p) => format!("STY    {}", p.to_asm()),
            Code::Tax(ref p) => format!("TAX    {}", p.to_asm()),
            Code::Txa(ref p) => format!("TXA    {}", p.to_asm()),
            Code::Sbc(ref p) => format!("SBC    {}", p.to_asm()),
            Code::Clc(ref p) => format!("CLC    {}", p.to_asm()),
            Code::Sec(ref p) => format!("SEC    {}", p.to_asm()),
            Code::Jsr(ref p) => format!("JSR    {}", p.to_asm()),
            Code::Jmp(ref p) => format!("JMP    {}", p.to_asm()),
            Code::Rts(ref p) => format!("RTS    {}", p.to_asm()),
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
            write!(asm, "    {}\n", code.to_asm())?;
        }
        Ok(asm)
    }
}
