use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use ast::{Literal, NameType, Type};

#[derive(Debug, Copy, Clone)]
pub enum Location {
    UndeterminedGlobal,
    Global(u16),
    FrameOffset(i8),
}

#[derive(Eq, PartialEq, Hash, Debug, Clone)]
pub struct SymbolRef(pub Arc<String>);

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: Arc<String>,
    pub location: Option<Location>,
    pub parameters: Vec<NameType>,
    pub return_type: Type,
    pub frame_size: i8,
}

pub type FunctionMetadataPtr = Arc<RwLock<FunctionMetadata>>;

#[derive(Debug, Clone)]
pub struct SymbolTable {
    parent: Option<Arc<RwLock<SymbolTable>>>,
    pub constants: HashMap<SymbolRef, Literal>,
    pub functions: HashMap<SymbolRef, FunctionMetadataPtr>,
    pub variables: HashMap<SymbolRef, (Type, Location)>,
    next_frame_offset: i8,
    next_block_index: usize,
}

impl SymbolTable {
    pub fn new() -> SymbolTable {
        SymbolTable {
            parent: None,
            constants: HashMap::new(),
            functions: HashMap::new(),
            variables: HashMap::new(),
            next_frame_offset: 0,
            next_block_index: 0,
        }
    }

    pub fn new_from_parent(parent: Arc<RwLock<SymbolTable>>, frame_offset: i8) -> SymbolTable {
        let mut symbol_table = SymbolTable::new();
        symbol_table.parent = Some(parent);
        symbol_table.next_frame_offset = frame_offset;
        symbol_table
    }

    pub fn new_run_block_name(&mut self) -> Arc<String> {
        if let Some(ref parent) = self.parent {
            parent.write().unwrap().new_run_block_name()
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
        parent_has_symbol || self.constants.contains_key(symbol_ref) || self.functions.contains_key(symbol_ref)
            || self.variables.contains_key(symbol_ref)
    }

    pub fn next_frame_offset(&mut self, local_size: usize) -> i8 {
        let result = self.next_frame_offset;
        self.next_frame_offset += local_size as i8;
        result
    }

    pub fn create_temporary(&mut self, typ: Type) -> SymbolRef {
        let next_location = self.next_frame_offset(typ.size());
        let symbol_ref = SymbolRef(Arc::new(format!("tmp#{}", next_location)));
        self.variables.insert(
            symbol_ref.clone(),
            (typ, Location::FrameOffset(next_location)),
        );
        symbol_ref
    }

    pub fn create_temporary_location(&mut self, typ: Type) -> Location {
        let symbol = self.create_temporary(typ);
        self.variable(&symbol).unwrap().1
    }

    pub fn function(&self, symbol_ref: &SymbolRef) -> Option<FunctionMetadataPtr> {
        let function = self.functions.get(symbol_ref);
        if function.is_some() {
            function.map(|f| f.clone())
        } else if let &Some(ref parent) = &self.parent {
            parent.read().unwrap().function(symbol_ref)
        } else {
            None
        }
    }

    pub fn variable(&self, symbol_ref: &SymbolRef) -> Option<(Type, Location)> {
        let variable = self.variables.get(symbol_ref);
        if variable.is_some() {
            variable.map(|v| *v)
        } else if let &Some(ref parent) = &self.parent {
            parent.read().unwrap().variable(symbol_ref)
        } else {
            None
        }
    }
}
