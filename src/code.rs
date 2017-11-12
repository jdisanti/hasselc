use std::fmt;

#[derive(Clone)]
pub enum Global {
    Resolved(u16),
    UnresolvedBlock,
    UnresolvedName(String),
}

impl fmt::Debug for Global {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Global::Resolved(val) => write!(f, "${:04X}", val)?,
            Global::UnresolvedBlock => write!(f, "UNRESOLVED_BLOCK")?,
            Global::UnresolvedName(ref name) => write!(f, "{}", name)?,
        }
        Ok(())
    }
}

#[derive(Clone)]
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

impl fmt::Debug for Parameter {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Parameter::Implicit => {}
            Parameter::Accumulator => write!(f, "A")?,
            Parameter::Immediate(val) => write!(f, "#{}", val)?,
            Parameter::ZeroPage(offset) => write!(f, "${:02X}", offset)?,
            Parameter::ZeroPageX(offset) => write!(f, "${:02X},X", offset)?,
            Parameter::ZeroPageY(offset) => write!(f, "${:02X},Y", offset)?,
            Parameter::Absolute(ref gbl) => write!(f, "{:?}", gbl)?,
            _ => unimplemented!(),
        }
        Ok(())
    }
}

#[derive(Clone)]
pub enum Code {
    Adc(Parameter),
    Lda(Parameter),
    Sta(Parameter),
    Ldx(Parameter),
    Stx(Parameter),
    Tax(Parameter),
    Txa(Parameter),
    Sbc(Parameter),
    Clc(Parameter),
    Sec(Parameter),
    Jsr(Parameter),
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
            Code::Tax(ref p) => p,
            Code::Txa(ref p) => p,
            Code::Sbc(ref p) => p,
            Code::Clc(ref p) => p,
            Code::Sec(ref p) => p,
            Code::Jsr(ref p) => p,
            Code::Rts(ref p) => p,
        }
    }
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Code::Adc(ref p) => write!(f, "ADC {:?}", p)?,
            Code::Lda(ref p) => write!(f, "LDA {:?}", p)?,
            Code::Sta(ref p) => write!(f, "STA {:?}", p)?,
            Code::Ldx(ref p) => write!(f, "LDX {:?}", p)?,
            Code::Stx(ref p) => write!(f, "STX {:?}", p)?,
            Code::Tax(ref p) => write!(f, "TAX {:?}", p)?,
            Code::Txa(ref p) => write!(f, "TXA {:?}", p)?,
            Code::Sbc(ref p) => write!(f, "SBC {:?}", p)?,
            Code::Clc(ref p) => write!(f, "CLC {:?}", p)?,
            Code::Sec(ref p) => write!(f, "SEC {:?}", p)?,
            Code::Jsr(ref p) => write!(f, "JSR {:?}", p)?,
            Code::Rts(ref p) => write!(f, "RTS {:?}", p)?,
        }
        Ok(())
    }
}

pub struct CodeBlock {
    pub location: Global,
    pub name: Option<String>,
    pub body: Vec<Code>,
}

impl CodeBlock {
    pub fn new(name: Option<String>, location: Option<u16>) -> CodeBlock {
        CodeBlock {
            location: match location {
                Some(val) => Global::Resolved(val),
                None => Global::UnresolvedBlock,
            },
            name: name,
            body: Vec::new(),
        }
    }
}

impl fmt::Debug for CodeBlock {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        writeln!(f, "CodeBlock {{")?;
        writeln!(f, "    location: {:?}", self.location)?;
        writeln!(f, "    name: {:?}", self.name)?;
        writeln!(f, "    body: [")?;
        for code in &self.body {
            writeln!(f, "        {:?}", code)?;
        }
        writeln!(f, "    ]")?;
        write!(f, "}}")?;
        Ok(())
    }
}
