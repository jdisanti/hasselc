use std::fmt::Write;
use symbol_table::{SymbolName, SymbolRef, SymbolTable};
use error;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Global {
    Resolved(u16),
    UnresolvedBlock,
    UnresolvedSymbol(SymbolRef),
    UnresolvedSymbolHighByte(SymbolRef),
    UnresolvedSymbolLowByte(SymbolRef),
}

impl Global {
    fn to_asm(&self, global_symbol_table: &SymbolTable) -> String {
        match *self {
            Global::Resolved(val) => format!("${:04X}", val),
            Global::UnresolvedBlock => String::from("UNRESOLVED_BLOCK"),
            Global::UnresolvedSymbol(symbol) => format!("{}", global_symbol_table.get_symbol_name(symbol).unwrap()),
            Global::UnresolvedSymbolHighByte(symbol) => {
                format!("#>{}", global_symbol_table.get_symbol_name(symbol).unwrap())
            }
            Global::UnresolvedSymbolLowByte(symbol) => {
                format!("#<{}", global_symbol_table.get_symbol_name(symbol).unwrap())
            }
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
    fn to_asm(&self, global_symbol_table: &SymbolTable) -> String {
        match *self {
            Parameter::Implicit => String::from(""),
            Parameter::Accumulator => String::from("A"),
            Parameter::Immediate(val) => format!("#{}", val),
            Parameter::ZeroPage(offset) => format!("${:02X}", offset),
            Parameter::ZeroPageX(offset) => format!("${:02X}, X", offset),
            Parameter::ZeroPageY(offset) => format!("${:02X}, Y", offset),
            Parameter::Absolute(ref gbl) => gbl.to_asm(global_symbol_table),
            Parameter::AbsoluteY(ref gbl) => format!("{}, Y", gbl.to_asm(global_symbol_table)),
            Parameter::IndirectX(val) => format!("(${:02X}, X)", val),
            Parameter::IndirectY(val) => format!("(${:02X}), Y", val),
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Code {
    Adc(Parameter),
    And(Parameter),
    Beq(Parameter),
    Clc(Parameter),
    Cmp(Parameter),
    Eor(Parameter),
    Jmp(Parameter),
    Jsr(Parameter),
    Lda(Parameter),
    Ldx(Parameter),
    Ldy(Parameter),
    Php(Parameter),
    Pla(Parameter),
    Ror(Parameter),
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

    Comment(String),
}

impl Code {
    pub fn parameter(&self) -> &Parameter {
        use self::Code::*;
        match *self {
            Adc(ref p) | And(ref p) | Beq(ref p) | Clc(ref p) | Cmp(ref p) | Eor(ref p) | Jmp(ref p) | Jsr(ref p)
            | Lda(ref p) | Ldx(ref p) | Ldy(ref p) | Php(ref p) | Pla(ref p) | Ror(ref p) | Rts(ref p) | Sbc(ref p)
            | Sec(ref p) | Sta(ref p) | Stx(ref p) | Sty(ref p) | Tax(ref p) | Tay(ref p) | Txa(ref p) | Tya(ref p) => {
                p
            }
            Comment(_) => unreachable!(),
        }
    }

    pub fn is_branch(&self) -> bool {
        match *self {
            Code::Beq(_) | Code::Jsr(_) | Code::Rts(_) | Code::Jmp(_) => true,
            _ => false,
        }
    }

    pub fn to_asm(&self, global_symbol_table: &SymbolTable) -> String {
        match *self {
            Code::Adc(ref p) => format!("ADC\t{}", p.to_asm(global_symbol_table)),
            Code::And(ref p) => format!("AND\t{}", p.to_asm(global_symbol_table)),
            Code::Beq(ref p) => format!("BEQ\t{}", p.to_asm(global_symbol_table)),
            Code::Clc(ref p) => format!("CLC\t{}", p.to_asm(global_symbol_table)),
            Code::Cmp(ref p) => format!("CMP\t{}", p.to_asm(global_symbol_table)),
            Code::Eor(ref p) => format!("EOR\t{}", p.to_asm(global_symbol_table)),
            Code::Jmp(ref p) => format!("JMP\t{}", p.to_asm(global_symbol_table)),
            Code::Jsr(ref p) => format!("JSR\t{}", p.to_asm(global_symbol_table)),
            Code::Lda(ref p) => format!("LDA\t{}", p.to_asm(global_symbol_table)),
            Code::Ldx(ref p) => format!("LDX\t{}", p.to_asm(global_symbol_table)),
            Code::Ldy(ref p) => format!("LDY\t{}", p.to_asm(global_symbol_table)),
            Code::Php(ref p) => format!("PHP\t{}", p.to_asm(global_symbol_table)),
            Code::Pla(ref p) => format!("PLA\t{}", p.to_asm(global_symbol_table)),
            Code::Ror(ref p) => format!("ROR\t{}", p.to_asm(global_symbol_table)),
            Code::Rts(ref p) => format!("RTS\t{}", p.to_asm(global_symbol_table)),
            Code::Sbc(ref p) => format!("SBC\t{}", p.to_asm(global_symbol_table)),
            Code::Sec(ref p) => format!("SEC\t{}", p.to_asm(global_symbol_table)),
            Code::Sta(ref p) => format!("STA\t{}", p.to_asm(global_symbol_table)),
            Code::Stx(ref p) => format!("STX\t{}", p.to_asm(global_symbol_table)),
            Code::Sty(ref p) => format!("STY\t{}", p.to_asm(global_symbol_table)),
            Code::Tax(ref p) => format!("TAX\t{}", p.to_asm(global_symbol_table)),
            Code::Tay(ref p) => format!("TAY\t{}", p.to_asm(global_symbol_table)),
            Code::Txa(ref p) => format!("TXA\t{}", p.to_asm(global_symbol_table)),
            Code::Tya(ref p) => format!("TYA\t{}", p.to_asm(global_symbol_table)),
            Code::Comment(ref msg) => format!("; {}", msg),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CodeBlock {
    pub location: Global,
    pub name: SymbolName,
    pub symbol: SymbolRef,
    pub body: Vec<Code>,
}

impl CodeBlock {
    pub fn new(name: SymbolName, symbol: SymbolRef, location: Option<u16>) -> CodeBlock {
        CodeBlock {
            location: match location {
                Some(val) => Global::Resolved(val),
                None => Global::UnresolvedBlock,
            },
            name: name,
            symbol: symbol,
            body: Vec::new(),
        }
    }

    pub fn to_asm(&self, global_symbol_table: &SymbolTable) -> error::Result<String> {
        let mut asm = String::new();
        if let Global::Resolved(addr) = self.location {
            write!(asm, ".org ${:04X}\n\n", addr)?;
        }
        write!(asm, "\n{}:\n", self.name)?;
        for code in &self.body {
            write!(asm, "\t{}\n", code.to_asm(global_symbol_table))?;
        }
        Ok(asm)
    }
}

pub fn to_asm(global_symbol_table: &SymbolTable, blocks: &[CodeBlock]) -> error::Result<String> {
    let mut asm = String::new();
    for block in blocks {
        asm.push_str(&block.to_asm(global_symbol_table).unwrap());
    }
    for (symbol_ref, text) in global_symbol_table.texts() {
        let symbol_name = global_symbol_table.get_symbol_name(symbol_ref).unwrap();
        write!(asm, "\n{}:\t.byte\t\"{}\",0\n", symbol_name, text)?;
    }
    Ok(asm)
}
