use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ast::{NameType, Type};

#[derive(Debug, Copy, Clone)]
pub enum Location {
    UndeterminedGlobal,
    Global(u16),
    FrameOffset(i8),
}

pub type SymbolRef = Arc<String>;

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: Arc<String>,
    pub location: Option<Location>,
    pub parameters: Vec<NameType>,
    pub return_type: Type,
    pub frame_size: i8,
}

pub type FunctionMetadataPtr = Arc<RwLock<FunctionMetadata>>;

#[derive(Debug, Copy, Clone)]
pub struct Variable {
    pub type_name: Type,
    pub location: Location,
}

impl Variable {
    pub fn new(type_name: Type, location: Location) -> Variable {
        Variable {
            type_name: type_name,
            location: location,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub enum ConstantValue {
    U8(u8),
    U16(u16),
}

impl ConstantValue {
    pub fn as_u8(&self) -> u8 {
        match *self {
            ConstantValue::U8(val) => val,
            _ => panic!("expected u8"),
        }
    }

    pub fn as_u16(&self) -> u16 {
        match *self {
            ConstantValue::U16(val) => val,
            _ => panic!("expected u16"),
        }
    }
}

#[derive(Clone, Debug)]
enum Symbol {
    Constant(ConstantValue),
    Variable(Variable),
    Function(FunctionMetadataPtr),
}

#[derive(Default, Debug, Clone)]
pub struct SymbolTable {
    parent: Option<Arc<RwLock<SymbolTable>>>,
    symbols: HashMap<SymbolRef, Symbol>,
    next_frame_offset: i8,
    next_block_index: usize,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        Default::default()
    }

    pub fn new_from_parent(parent: Arc<RwLock<SymbolTable>>, frame_offset: i8) -> SymbolTable {
        let mut symbol_table = SymbolTable::new();
        symbol_table.parent = Some(parent);
        symbol_table.next_frame_offset = frame_offset;
        symbol_table
    }

    pub fn new_block_name(&mut self) -> Arc<String> {
        if let Some(ref parent) = self.parent {
            parent.write().unwrap().new_block_name()
        } else {
            let result = Arc::new(format!("__L{:04X}_", self.next_block_index));
            self.next_block_index += 1;
            result
        }
    }

    pub fn has_symbol(&self, symbol_ref: &SymbolRef) -> bool {
        let parent_has_symbol = if let Some(ref parent) = self.parent {
            parent.read().unwrap().has_symbol(symbol_ref)
        } else {
            false
        };
        parent_has_symbol || self.symbols.contains_key(symbol_ref)
    }

    pub fn next_frame_offset(&mut self, local_size: usize) -> i8 {
        let result = self.next_frame_offset;
        self.next_frame_offset += local_size as i8;
        result
    }

    pub fn create_temporary(&mut self, typ: Type) -> SymbolRef {
        let next_location = self.next_frame_offset(typ.size());
        let symbol_ref = SymbolRef::new(format!("tmp#{}", next_location));
        self.symbols.insert(
            SymbolRef::clone(&symbol_ref),
            Symbol::Variable(Variable::new(typ, Location::FrameOffset(next_location))),
        );
        symbol_ref
    }

    pub fn create_temporary_location(&mut self, typ: Type) -> Location {
        let symbol = self.create_temporary(typ);
        self.variable(&symbol).unwrap().location
    }

    fn insert(&mut self, symbol_ref: SymbolRef, symbol: Symbol) -> bool {
        if self.has_symbol(&symbol_ref) {
            false
        } else {
            self.symbols.insert(symbol_ref, symbol);
            true
        }
    }

    pub fn insert_constant(&mut self, symbol_ref: SymbolRef, value: ConstantValue) -> bool {
        self.insert(symbol_ref, Symbol::Constant(value))
    }

    pub fn constant(&self, symbol_ref: &SymbolRef) -> Option<ConstantValue> {
        match self.symbols.get(symbol_ref) {
            Some(&Symbol::Constant(ref value)) => Some(*value),
            _ => if let Some(ref parent) = self.parent {
                parent.read().unwrap().constant(symbol_ref)
            } else {
                None
            },
        }
    }

    pub fn insert_function(&mut self, symbol_ref: SymbolRef, metadata: FunctionMetadataPtr) -> bool {
        self.insert(symbol_ref, Symbol::Function(metadata))
    }

    pub fn function(&self, symbol_ref: &SymbolRef) -> Option<FunctionMetadataPtr> {
        match self.symbols.get(symbol_ref) {
            Some(&Symbol::Function(ref function_metadata)) => Some(FunctionMetadataPtr::clone(function_metadata)),
            _ => if let Some(ref parent) = self.parent {
                parent.read().unwrap().function(symbol_ref)
            } else {
                None
            },
        }
    }

    pub fn insert_variable(&mut self, symbol_ref: SymbolRef, variable: Variable) -> bool {
        self.insert(symbol_ref, Symbol::Variable(variable))
    }

    pub fn variable(&self, symbol_ref: &SymbolRef) -> Option<Variable> {
        match self.symbols.get(symbol_ref) {
            Some(&Symbol::Variable(ref variable)) => Some(*variable),
            _ => if let Some(ref parent) = self.parent {
                parent.read().unwrap().variable(symbol_ref)
            } else {
                None
            },
        }
    }

    pub fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a> {
        Box::new(self.symbols.values().filter_map(|symbol| match *symbol {
            Symbol::Variable(ref variable) => Some(variable),
            _ => None,
        }))
    }
}
