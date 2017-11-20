use std::fmt;
use std::sync::Arc;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Index {
    Immediate(u8),
    DataStack(u8),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Location {
    DataStackOffset(i8),
    FrameOffset(Arc<String>, i8),
    FrameOffsetBeforeCall(Arc<String>, Arc<String>, i8),
    Global(u16),
    GlobalIndexed(u16, Index),
    UnresolvedBlock,
    UnresolvedGlobal(Arc<String>),
    UnresolvedGlobalIndexed(Arc<String>, Index),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Value {
    Immediate(u8),
    Memory(Location),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum SPOffset {
    Immediate(i8),
    FrameSize(Arc<String>),
    NegativeFrameSize(Arc<String>),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CopyData {
    pub destination: Location,
    pub value: Value,
}

impl CopyData {
    pub fn new(destination: Location, value: Value) -> CopyData {
        CopyData {
            destination: destination,
            value: value,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BinaryOpData {
    pub destination: Location,
    pub left: Value,
    pub right: Value,
}

impl BinaryOpData {
    pub fn new(destination: Location, left: Value, right: Value) -> BinaryOpData {
        BinaryOpData {
            destination: destination,
            left: left,
            right: right,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BranchIfZeroData {
    pub value: Value,
    pub destination: Arc<String>,
}

impl BranchIfZeroData {
    pub fn new(value: Value, destination: Arc<String>) -> BranchIfZeroData {
        BranchIfZeroData {
            value: value,
            destination: destination,
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub enum Statement {
    Add(BinaryOpData),
    AddToDataStackPointer(SPOffset),
    BranchIfZero(BranchIfZeroData),
    Compare(BinaryOpData),
    Copy(CopyData),
    GoTo(Arc<String>),
    JumpRoutine(Location),
    Return,
    Subtract(BinaryOpData),
}

impl Statement {
    pub fn is_branch(&self) -> bool {
        match *self {
            Statement::Add { .. } => false,
            Statement::AddToDataStackPointer { .. } => false,
            Statement::BranchIfZero(_) => true,
            Statement::Compare(_) => false,
            Statement::Copy { .. } => false,
            Statement::GoTo { .. } => true,
            Statement::JumpRoutine { .. } => true,
            Statement::Return { .. } => true,
            Statement::Subtract { .. } => false,
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
            Statement::BranchIfZero(ref data) => write!(f, "branch to {:?} if {:?} == 0", data.destination, data.value)?,
            Statement::Compare(ref data) => write!(f, "compare {:?} to {:?}", data.left, data.right)?,
            Statement::Copy(ref data) => write!(f, "copy {:?} => {:?}", data.value, data.destination)?,
            Statement::GoTo(ref name) => write!(f, "goto {}", name)?,
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
