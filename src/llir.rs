use std::fmt;
use std::sync::Arc;
use src_tag::SrcTag;
use symbol_table::SymbolRef;

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

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Immediate(u8),
    Memory(Location),
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

#[derive(Clone, Eq, PartialEq)]
pub enum Statement {
    Add(BinaryOpData),
    AddToDataStackPointer(SPOffset),
    BranchIfZero(BranchIfZeroData),
    CompareEq(BinaryOpData),
    CompareNotEq(BinaryOpData),
    CompareLt(BinaryOpData),
    CompareGte(BinaryOpData),
    Copy(CopyData),
    GoTo(GoToData),
    JumpRoutine(JumpRoutineData),
    Return,
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
            Statement::Return => write!(f, "rts")?,
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
