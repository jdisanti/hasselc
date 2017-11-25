use std::fmt;
use src_tag::{SrcTag, SrcTagged};
use symbol_table::{SymbolName, SymbolRef};
use types::TypedValue;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Location {
    DataStackOffset(i8),
    FrameOffset(SymbolRef, i8),
    FrameOffsetBeforeCall(SymbolRef, SymbolRef, i8),
    Global(u16),
    GlobalIndexed(u16, Box<Value>),
    UnresolvedGlobal(SymbolRef),
    UnresolvedGlobalIndexed(SymbolRef, Box<Value>),
    UnresolvedGlobalOffset(SymbolRef, i8),
    UnresolvedBlock,
}

impl Location {
    pub fn offset(&self, offset_by: i8) -> Location {
        use self::Location::*;
        match *self {
            DataStackOffset(offset) => DataStackOffset(offset + offset_by),
            FrameOffset(symbol, offset) => FrameOffset(symbol, offset + offset_by),
            FrameOffsetBeforeCall(sym1, sym2, offset) => FrameOffsetBeforeCall(sym1, sym2, offset + offset_by),
            Global(offset) => Global((offset as isize + offset_by as isize) as u16),
            GlobalIndexed(offset, ref index) => GlobalIndexed(offset + 1, index.clone()),
            // TODO: Are these unresolved global's useful? They don't seem to work
            UnresolvedGlobal(symbol) => UnresolvedGlobalOffset(symbol, 1),
            UnresolvedGlobalIndexed(_, _) => {
                unimplemented!("this will probably require a refactor to output more statements")
            }
            UnresolvedGlobalOffset(_, _) => {
                unimplemented!("this will probably require a refactor to output more statements")
            }
            UnresolvedBlock => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Immediate(TypedValue),
    Memory(Location),
}

impl Value {
    pub fn high_byte(value: &Value) -> Value {
        use self::Value::*;
        // 16-bit values on the 6502 are in little-endian
        match *value {
            Immediate(TypedValue::U16(val)) => Immediate(TypedValue::U8((val >> 8) as u8)),
            Immediate(_) => unreachable!(),
            Memory(ref location) => Memory(location.offset(1)),
        }
    }

    pub fn low_byte(value: &Value) -> Value {
        use self::Value::*;
        // 16-bit values on the 6502 are in little-endian
        match *value {
            Immediate(TypedValue::U8(_)) | Memory(_) => value.clone(),
            Immediate(TypedValue::U16(val)) => Immediate(TypedValue::U8(val as u8)),
            Immediate(_) => unreachable!(),
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
