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

#[derive(Clone, Eq, PartialEq)]
pub enum Statement {
    AddToDataStackPointer(SPOffset),
    Copy(CopyData),
    Add(BinaryOpData),
    Subtract(BinaryOpData),
    JumpRoutine(Location),
    GoTo(Arc<String>),
    Return,
}

impl Statement {
    pub fn is_branch(&self) -> bool {
        match *self {
            Statement::AddToDataStackPointer { .. } => false,
            Statement::Copy { .. } => false,
            Statement::Add { .. } => false,
            Statement::Subtract { .. } => false,
            Statement::JumpRoutine { .. } => true,
            Statement::GoTo { .. } => true,
            Statement::Return { .. } => true,
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Statement::AddToDataStackPointer(ref offset) => write!(f, "add_dsp {:?}", offset)?,
            Statement::Copy(ref data) => write!(f, "copy {:?} => {:?}", data.value, data.destination)?,
            Statement::Add(ref data) => write!(
                f,
                "add {:?} + {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::Subtract(ref data) => write!(
                f,
                "subtract {:?} - {:?} => {:?}",
                data.left,
                data.right,
                data.destination
            )?,
            Statement::JumpRoutine(ref location) => write!(f, "jsr {:?}", location)?,
            Statement::GoTo(ref name) => write!(f, "goto {}", name)?,
            Statement::Return => write!(f, "rts")?,
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub location: Location,
    pub name: Option<Arc<String>>,
    pub statements: Vec<Statement>,
    pub frame_size: i8,
}

impl Block {
    pub fn new(name: Option<Arc<String>>, location: Location) -> Block {
        Block {
            location: location,
            name: name,
            statements: Vec::new(),
            frame_size: 0,
        }
    }
}
