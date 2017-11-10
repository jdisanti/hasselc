use std::collections::HashMap;
use ast::{BinaryOperator, Literal, NameType, Type};

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct SymbolRef(pub String);

pub enum Location {
    UndeterminedGlobal,
    Global(u16),
    StackOffset(u8),
}

pub struct SymbolTable {
    pub constants: HashMap<SymbolRef, Literal>,
    pub functions: HashMap<SymbolRef, Location>,
    pub variables: HashMap<SymbolRef, Location>,
}

impl SymbolTable {
    fn new() -> SymbolTable {
        SymbolTable {
            constants: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }
}

pub enum Expr {
    Number(i32),
    Symbol(SymbolRef),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    }
}

pub enum Statement {
    Assign {
        symbol: SymbolRef,
        value: Expr,
    },
    Break,
    Call {
        symbol: SymbolRef,
        arguments: Vec<Expr>,
    },
    LeftShift(SymbolRef),
    RotateLeft(SymbolRef),
    RotateRight(SymbolRef),
    Return(Expr),
}

// Intermediate representation
pub enum IR {
    AnonymousBlock {
        location: Option<Location>,
        body: Vec<Statement>,
    },
    FunctionBlock {
        location: Option<Location>,
        name: String,
        local_symbols: SymbolTable,
        parameters: Vec<NameType>,
        body: Vec<Statement>,
        return_type: Type,
    }
}