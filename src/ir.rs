use std::collections::HashMap;
use std::rc::Rc;
use std::cell::RefCell;
use ast::{BinaryOperator, Literal, NameType, Type};

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct SymbolRef(pub String);

#[derive(Debug, Copy, Clone)]
pub enum Location {
    UndeterminedGlobal,
    Global(u16),
    StackOffset(u8),
}

#[derive(Debug)]
pub struct SymbolTable {
    parent: Option<Rc<RefCell<SymbolTable>>>,
    pub constants: HashMap<SymbolRef, Literal>,
    pub functions: HashMap<SymbolRef, Location>,
    pub variables: HashMap<SymbolRef, (Type, Location)>,
    next_local_stack_offset: u8,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable {
            parent: None,
            constants: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            next_local_stack_offset: 0,
        }
    }

    pub fn new_from_parent(parent: Rc<RefCell<SymbolTable>>) -> SymbolTable {
        let mut symbol_table = SymbolTable::new();
        symbol_table.parent = Some(parent);
        symbol_table
    }

    pub fn next_local_stack_offset(&mut self, local_size: usize) -> u8 {
        let result = self.next_local_stack_offset;
        self.next_local_stack_offset += local_size as u8;
        result
    }

    pub fn create_temporary(&mut self, typ: Type) -> SymbolRef {
        let next_location = self.next_local_stack_offset(typ.size());
        let symbol_ref = SymbolRef(format!("tmp#{}", next_location));
        self.variables.insert(symbol_ref.clone(), (typ, Location::StackOffset(next_location)));
        symbol_ref
    } 

    pub fn parent(&self) -> Option<Rc<RefCell<SymbolTable>>> {
        self.parent.clone()
    }

    pub fn function(&self, symbol_ref: &SymbolRef) -> Option<&Location> {
        self.functions.get(symbol_ref)
    }

    pub fn variable(&self, symbol_ref: &SymbolRef) -> Option<(Type, Location)> {
        let variable = self.variables.get(symbol_ref);
        if variable.is_some() {
            variable.map(|v| *v)
        } else if let &Some(ref parent) = &self.parent {
            parent.borrow().variable(symbol_ref)
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum Expr {
    Number(i32),
    Symbol(SymbolRef),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Call {
        symbol: SymbolRef,
        arguments: Vec<Expr>,
    },
}

#[derive(Debug)]
pub enum Statement {
    Assign {
        symbol: SymbolRef,
        value: Expr,
    },
    Break,
    Call(Expr),
    LeftShift(SymbolRef),
    RotateLeft(SymbolRef),
    RotateRight(SymbolRef),
    Return(Expr),
}

// Intermediate representation
#[derive(Debug)]
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

impl IR {
    pub fn new_anonymous_block() -> IR {
        IR::AnonymousBlock {
            location: None,
            body: Vec::new(),
        }
    }

    pub fn new_function_block(parent_symbol_table: Rc<RefCell<SymbolTable>>,
            name: String, parameters: Vec<NameType>, return_type: Type) -> IR {
        let mut symbol_table = SymbolTable::new_from_parent(parent_symbol_table);

        for parameter in &parameters {
            let next_location = symbol_table.next_local_stack_offset(parameter.type_name.size());
            symbol_table.variables.insert(SymbolRef(parameter.name.clone()),
                (parameter.type_name, Location::StackOffset(next_location)));
        }

        IR::FunctionBlock {
            location: None,
            name: name,
            local_symbols: symbol_table,
            parameters: parameters,
            body: Vec::new(),
            return_type: return_type,
        }
    }

    pub fn is_empty_anonymous(&self) -> bool {
        match self {
            &IR::AnonymousBlock { ref body, .. } => body.is_empty(),
            &IR::FunctionBlock { ref body, .. } => false,
        }
    }

    pub fn symbol_table_mut(&mut self) -> &mut SymbolTable {
        match self {
            &mut IR::AnonymousBlock { .. } => unreachable!(),
            &mut IR::FunctionBlock { ref mut local_symbols, .. } => local_symbols,
        }
    }

    pub fn location(&self) -> &Option<Location> {
        match *self {
            IR::AnonymousBlock { ref location, .. } => location,
            IR::FunctionBlock { ref location, .. } => location,
        }
    }

    pub fn has_location(&self) -> bool {
        match *self {
            IR::AnonymousBlock { ref location, .. } => location.is_some(),
            IR::FunctionBlock { ref location, .. } => location.is_some(),
        }
    }

    pub fn set_location(&mut self, new_location: Location) {
        match self {
            &mut IR::AnonymousBlock { ref mut location, .. } => { location.get_or_insert(new_location); }
            &mut IR::FunctionBlock { ref mut location, .. } => { location.get_or_insert(new_location); }
        }
    }

    pub fn body_mut<'a>(&'a mut self) -> &'a mut Vec<Statement> {
        match self {
            &mut IR::AnonymousBlock { ref mut body, .. } => body,
            &mut IR::FunctionBlock { ref mut body, .. } => body,
        }
    }
}