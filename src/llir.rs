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

#[derive(Clone, Eq, PartialEq)]
pub enum Statement {
    AddToDataStackPointer(SPOffset),
    Store { dest: Location, value: Value },
    Add {
        dest: Location,
        left: Value,
        right: Value,
    },
    Subtract {
        dest: Location,
        left: Value,
        right: Value,
    },
    JumpRoutine { location: Location },
    Return,
}

impl Statement {
    pub fn is_branch(&self) -> bool {
        match *self {
            Statement::AddToDataStackPointer { .. } => false,
            Statement::Store { .. } => false,
            Statement::Add { .. } => false,
            Statement::Subtract { .. } => false,
            Statement::JumpRoutine { .. } => true,
            Statement::Return { .. } => true,
        }
    }
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Statement::AddToDataStackPointer(ref offset) => write!(f, "add_dsp {:?}", offset)?,
            Statement::Store {
                ref dest,
                ref value,
            } => write!(f, "store {:?} => {:?}", value, dest)?,
            Statement::Add {
                ref dest,
                ref left,
                ref right,
            } => write!(f, "store {:?} + {:?} => {:?}", left, right, dest)?,
            Statement::Subtract {
                ref dest,
                ref left,
                ref right,
            } => write!(f, "store {:?} - {:?} => {:?}", left, right, dest)?,
            Statement::JumpRoutine { ref location } => write!(f, "jsr {:?}", location)?,
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
