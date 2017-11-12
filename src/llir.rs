use std::fmt;

#[derive(Debug, Copy, Clone)]
pub enum Index {
    Immediate(u8),
    DataStack(u8),
}

#[derive(Debug, Clone)]
pub enum Location {
    DataStackOffset(i8),
    FrameOffset(String, i8),
    Global(u16),
    GlobalIndexed(u16, Index),
    UnresolvedBlock,
    UnresolvedGlobal(String),
    UnresolvedGlobalIndexed(String, Index),
}

#[derive(Debug)]
pub enum Value {
    Immediate(u8),
    Memory(Location),
}

#[derive(Debug)]
pub enum SPOffset {
    Immediate(i8),
    FrameSize(String),
    NegativeFrameSize(String),
}

pub enum Statement {
    AddToDataStackPointer(SPOffset),
    Store { dest: Location, value: Value },
    Add { dest: Location, left: Value, right: Value },
    Subtract { dest: Location, left: Value, right: Value },
    JumpRoutine { location: Location },
    Return,
}

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match *self {
            Statement::AddToDataStackPointer(ref offset) => { write!(f, "add_dsp {:?}", offset); }
            Statement::Store { ref dest, ref value } => { write!(f, "store {:?} => {:?}", value, dest); }
            Statement::Add { ref dest, ref left, ref right } => { write!(f, "store {:?} + {:?} => {:?}", left, right, dest); }
            Statement::Subtract { ref dest, ref left, ref right } => { write!(f, "store {:?} - {:?} => {:?}", left, right, dest); }
            Statement::JumpRoutine { ref location } => { write!(f, "jsr {:?}", location); }
            Statement::Return => { write!(f, "rts"); }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Block {
    pub location: Location,
    pub name: Option<String>,
    pub statements: Vec<Statement>,
    pub frame_size: i8,
}

impl Block {
    pub fn new(name: Option<String>, location: Location) -> Block {
        Block {
            location: location,
            name: name,
            statements: Vec::new(),
            frame_size: 0,
        }
    }
}