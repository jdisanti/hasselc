use std::fmt;
use std::sync::Arc;
use src_tag::{SrcTag, SrcTagged};
use symbol_table::{SymbolName, SymbolRef};
use types::{AddressOrSymbol, Type, TypedValue};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Location {
    DataStackOffset(i8),
    FrameOffset(SymbolRef, i8),
    FrameOffsetIndirect(SymbolRef, i8),
    FrameOffsetBeforeCall(SymbolRef, SymbolRef, i8),
    Global(u16),
    GlobalIndexed(u16, Box<Value>),
    UnresolvedGlobal(SymbolRef),
    UnresolvedGlobalIndexed(SymbolRef, Box<Value>),
    UnresolvedGlobalLowByte(SymbolRef),
    UnresolvedGlobalHighByte(SymbolRef),
    UnresolvedBlock,
}

impl Location {
    pub fn low_byte(&self) -> Location {
        use self::Location::*;
        match *self {
            UnresolvedGlobal(symbol) => UnresolvedGlobalLowByte(symbol),
            _ => self.clone(),
        }
    }

    pub fn high_byte(&self) -> Location {
        use self::Location::*;
        match *self {
            DataStackOffset(offset) => DataStackOffset(offset + 1),
            FrameOffset(symbol, offset) => FrameOffset(symbol, offset + 1),
            FrameOffsetBeforeCall(sym1, sym2, offset) => FrameOffsetBeforeCall(sym1, sym2, offset + 1),
            Global(offset) => Global((offset as isize + 1) as u16),
            GlobalIndexed(offset, ref index) => GlobalIndexed(offset + 1, index.clone()),
            UnresolvedGlobal(symbol) => UnresolvedGlobalHighByte(symbol),
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Immediate(TypedValue),
    Memory(Type, Location),
}

impl Value {
    pub fn value_type(&self) -> Type {
        match *self {
            Value::Immediate(ref tv) => tv.get_type(),
            Value::Memory(typ, _) => typ,
        }
    }

    pub fn high_byte(value: &Value) -> Value {
        use self::Value::*;
        // 16-bit values on the 6502 are in little-endian
        match *value {
            Immediate(TypedValue::U16(val)) => Immediate(TypedValue::U8((val >> 8) as u8)),
            Immediate(TypedValue::ArrayU8(AddressOrSymbol::Address(addr))) => {
                Immediate(TypedValue::U8((addr >> 8) as u8))
            }
            Immediate(TypedValue::ArrayU8(AddressOrSymbol::Symbol(sym))) => {
                Value::Memory(Type::U8, Location::UnresolvedGlobalHighByte(sym))
            }
            Immediate(_) => unreachable!(),
            Memory(typ, ref location) => Memory(typ, location.high_byte()),
        }
    }

    pub fn low_byte(value: &Value) -> Value {
        use self::Value::*;
        // 16-bit values on the 6502 are in little-endian
        match *value {
            Immediate(TypedValue::U8(_)) => value.clone(),
            Immediate(TypedValue::U16(val)) => Immediate(TypedValue::U8(val as u8)),
            Immediate(TypedValue::ArrayU8(AddressOrSymbol::Address(addr))) => Immediate(TypedValue::U8(addr as u8)),
            Immediate(TypedValue::ArrayU8(AddressOrSymbol::Symbol(sym))) => {
                Value::Memory(Type::U8, Location::UnresolvedGlobalLowByte(sym))
            }
            Immediate(_) => unreachable!(),
            Memory(typ, ref location) => Memory(typ, location.low_byte()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SPOffset {
    Immediate(i8),
    FrameSize(SymbolRef),
    NegativeFrameSize(SymbolRef),
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct CopyData {
    pub tag: SrcTag,
    pub destination: Location,
    pub value: Value,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub destination: Location,
    pub left: Value,
    pub right: Value,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct BranchIfZeroData {
    pub tag: SrcTag,
    pub value: Value,
    pub destination: SymbolRef,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: SymbolRef,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct InlineAsmData {
    pub tag: SrcTag,
    pub asm: Arc<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct JumpRoutineData {
    pub tag: SrcTag,
    pub destination: Location,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct ReturnData {
    pub tag: SrcTag,
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct AddToDataStackPointerData {
    pub tag: SrcTag,
    pub offset: SPOffset,
}

#[derive(Clone, Eq, PartialEq)]
pub enum Statement {
    Add(BinaryOpData),
    AddToDataStackPointer(AddToDataStackPointerData),
    BranchIfZero(BranchIfZeroData),
    CompareEq(BinaryOpData),
    CompareNotEq(BinaryOpData),
    CompareLt(BinaryOpData),
    CompareGte(BinaryOpData),
    Copy(CopyData),
    GoTo(GoToData),
    InlineAsm(InlineAsmData),
    JumpRoutine(JumpRoutineData),
    Return(ReturnData),
    Subtract(BinaryOpData),
}

impl Statement {
    pub fn is_branch(&self) -> bool {
        use self::Statement::*;
        match *self {
            BranchIfZero(_) | GoTo(_) | JumpRoutine { .. } | Return { .. } => true,
            _ => false,
        }
    }
}

impl SrcTagged for Statement {
    fn src_tag(&self) -> SrcTag {
        use self::Statement::*;
        match *self {
            Add(ref d) | CompareEq(ref d) | CompareNotEq(ref d) | CompareLt(ref d) | CompareGte(ref d)
            | Subtract(ref d) => d.tag,
            AddToDataStackPointer(ref d) => d.tag,
            BranchIfZero(ref d) => d.tag,
            Copy(ref d) => d.tag,
            GoTo(ref d) => d.tag,
            InlineAsm(ref d) => d.tag,
            JumpRoutine(ref d) => d.tag,
            Return(ref d) => d.tag,
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Statement::Add(ref data) => write!(
                f,
                "add {:?} + {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::AddToDataStackPointer(ref offset) => write!(f, "add_dsp {:?}", offset)?,
            Statement::BranchIfZero(ref data) => write!(
                f,
                "branch to {:?} if {:?} == 0",
                data.destination,
                data.value
            )?,
            Statement::CompareEq(ref data) => write!(
                f,
                "compare {:?} == {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::CompareNotEq(ref data) => write!(
                f,
                "compare {:?} != {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::CompareLt(ref data) => write!(
                f,
                "compare {:?} < {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::CompareGte(ref data) => write!(
                f,
                "compare {:?} >= {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::Copy(ref data) => write!(f, "copy {:?} => {:?}", data.value, data.destination)?,
            Statement::GoTo(ref data) => write!(f, "goto {}", data.destination)?,
            Statement::InlineAsm(_) => write!(f, "inline_asm")?,
            Statement::JumpRoutine(ref location) => write!(f, "jsr {:?}", location)?,
            Statement::Return(_) => write!(f, "rts")?,
            Statement::Subtract(ref data) => write!(
                f,
                "subtract {:?} - {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct RunBlock {
    pub name: SymbolName,
    pub symbol: SymbolRef,
    pub statements: Vec<Statement>,
}

impl RunBlock {
    pub fn new(name: SymbolName, symbol: SymbolRef) -> RunBlock {
        RunBlock {
            name: name,
            symbol: symbol,
            statements: Vec::new(),
        }
    }

    pub fn new_tup(tup: (SymbolName, SymbolRef)) -> RunBlock {
        RunBlock::new(tup.0, tup.1)
    }
}

#[derive(Debug, Clone)]
pub struct FrameBlock {
    pub name: SymbolName,
    pub symbol: SymbolRef,
    pub location: Location,
    pub runs: Vec<RunBlock>,
    pub frame_size: i8,
}

impl FrameBlock {
    pub fn new(name: SymbolName, symbol: SymbolRef, location: Location) -> FrameBlock {
        FrameBlock {
            name: name,
            symbol: symbol,
            location: location,
            runs: Vec::new(),
            frame_size: 0,
        }
    }
}
