use std::fmt;
use std::sync::Arc;
use src_tag::{SrcTag, SrcTagged};
use symbol_table::SymbolRef;
use types::TypedValue;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Index {
    Immediate(u8),
    DataStack(u8),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Location {
    DataStackOffset(i8),
    FrameOffset(SymbolRef, i8),
    FrameOffsetBeforeCall(SymbolRef, SymbolRef, i8),
    Global(u16),
    GlobalIndexed(u16, Index),
    UnresolvedBlock,
    UnresolvedGlobal(SymbolRef),
    UnresolvedGlobalIndexed(SymbolRef, Index),
}

impl Location {
    pub fn offset(&self, offset_by: i8) -> Location {
        use self::Location::*;
        match *self {
            DataStackOffset(offset) => DataStackOffset(offset + offset_by),
            FrameOffset(ref symbol, offset) => FrameOffset(SymbolRef::clone(symbol), offset + offset_by),
            FrameOffsetBeforeCall(ref sym1, ref sym2, offset) => FrameOffsetBeforeCall(
                SymbolRef::clone(sym1),
                SymbolRef::clone(sym2),
                offset + offset_by,
            ),
            Global(offset) => Global((offset as isize + offset_by as isize) as u16),
            GlobalIndexed(offset, index) => GlobalIndexed(
                offset,
                match index {
                    Index::Immediate(val) => Index::Immediate((val as isize + offset_by as isize) as u8),
                    Index::DataStack(val) => Index::DataStack((val as isize + offset_by as isize) as u8),
                },
            ),
            UnresolvedGlobal(ref symbol) => {
                UnresolvedGlobalIndexed(SymbolRef::clone(symbol), Index::Immediate(offset_by as u8))
            }
            UnresolvedGlobalIndexed(ref symbol, index) => UnresolvedGlobalIndexed(
                SymbolRef::clone(symbol),
                match index {
                    Index::Immediate(val) => Index::Immediate((val as isize + offset_by as isize) as u8),
                    Index::DataStack(val) => Index::DataStack((val as isize + offset_by as isize) as u8),
                },
            ),
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
    pub name: Arc<String>,
    pub statements: Vec<Statement>,
}

impl RunBlock {
    pub fn new(name: Arc<String>) -> RunBlock {
        RunBlock {
            name: name,
            statements: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FrameBlock {
    pub location: Location,
    pub name: Option<Arc<String>>,
    pub runs: Vec<RunBlock>,
    pub frame_size: i8,
}

impl FrameBlock {
    pub fn new(name: Option<Arc<String>>, location: Location) -> FrameBlock {
        FrameBlock {
            location: location,
            name: name,
            runs: Vec::new(),
            frame_size: 0,
        }
    }
}