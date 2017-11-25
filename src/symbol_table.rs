use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use parse::ast::NameType;
use types::{Type, TypedValue};
use std::fmt::Debug;

#[derive(Debug, Copy, Clone)]
pub enum Location {
    UndeterminedGlobal,
    Global(u16),
    FrameOffset(i8),
}

pub type SymbolRef = usize;
pub type SymbolName = Arc<String>;

#[derive(Debug, Clone)]
pub struct FunctionMetadata {
    pub name: SymbolName,
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

#[derive(Clone, Debug)]
enum Symbol {
    Constant(TypedValue),
    Variable(Variable),
    Function(FunctionMetadataPtr),
    Block,
}

#[derive(Default, Debug)]
struct SymbolMap {
    by_name: HashMap<SymbolName, SymbolRef>,
    by_ref: HashMap<SymbolRef, Symbol>,
    name_by_ref: HashMap<SymbolRef, SymbolName>,
}

impl SymbolMap {
    fn new() -> SymbolMap {
        Default::default()
    }

    fn insert(&mut self, name: Arc<String>, reference: SymbolRef, symbol: Symbol) -> Option<SymbolRef> {
        if self.by_name.contains_key(&name) || self.by_ref.contains_key(&reference) {
            None
        } else {
            self.by_name.insert(SymbolName::clone(&name), reference);
            self.by_ref.insert(reference, symbol);
            self.name_by_ref.insert(reference, name);
            Some(reference)
        }
    }

    fn find_by_name(&self, name: &Arc<String>) -> Option<&Symbol> {
        if let Some(symbol_ref) = self.by_name.get(name) {
            self.by_ref.get(symbol_ref)
        } else {
            None
        }
    }

    fn find_by_ref(&self, symbol_ref: SymbolRef) -> Option<&Symbol> {
        self.by_ref.get(&symbol_ref)
    }

    fn get_ref(&self, symbol_name: &SymbolName) -> Option<SymbolRef> {
        self.by_name.get(symbol_name).cloned()
    }

    fn get_name(&self, symbol_ref: SymbolRef) -> Option<SymbolName> {
        self.name_by_ref.get(&symbol_ref).cloned()
    }

    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a> {
        Box::new(self.by_ref.values().filter_map(|symbol| match *symbol {
            Symbol::Variable(ref variable) => Some(variable),
            _ => None,
        }))
    }
}

#[derive(Default, Debug)]
pub struct HandleGenerator {
    next_handle: usize,
}

impl HandleGenerator {
    pub fn new() -> HandleGenerator {
        HandleGenerator { next_handle: 0 }
    }

    pub fn new_handle(&mut self) -> usize {
        let result = self.next_handle;
        self.next_handle += 1;
        result
    }
}

pub trait SymbolTable: Send + Sync + Debug {
    fn new_block_name(&mut self) -> (SymbolName, SymbolRef);
    fn insert_block(&mut self, symbol_name: SymbolName) -> Option<SymbolRef>;

    fn has_symbol(&self, symbol_name: &SymbolName) -> bool;
    fn find_symbol(&self, symbol_name: &SymbolName) -> Option<SymbolRef>;
    fn get_symbol_name(&self, symbol_ref: SymbolRef) -> Option<SymbolName>;

    fn next_frame_offset(&mut self, local_size: usize) -> i8;

    fn create_temporary(&mut self, typ: Type) -> SymbolRef;
    fn create_temporary_location(&mut self, typ: Type) -> Location;

    fn insert_constant(&mut self, symbol_name: SymbolName, value: TypedValue) -> Option<SymbolRef>;
    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<TypedValue>;
    fn constant(&self, symbol_ref: SymbolRef) -> Option<TypedValue>;

    fn insert_function(&mut self, symbol_name: SymbolName, metadata: FunctionMetadataPtr) -> Option<SymbolRef>;
    fn function_by_name(&self, symbol_name: &SymbolName) -> Option<FunctionMetadataPtr>;
    fn function(&self, symbol_ref: SymbolRef) -> Option<FunctionMetadataPtr>;

    fn insert_variable(&mut self, symbol_name: SymbolName, variable: Variable) -> Option<SymbolRef>;
    fn variable_by_name(&self, symbol_name: &SymbolName) -> Option<Variable>;
    fn variable(&self, symbol_ref: SymbolRef) -> Option<Variable>;
    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a>;

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<Type>;
    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<Type>;

    fn handle_gen(&self) -> Arc<RwLock<HandleGenerator>>;
}

#[derive(Debug)]
pub struct DefaultSymbolTable {
    handle_gen: Arc<RwLock<HandleGenerator>>,
    symbols: SymbolMap,
    next_frame_offset: i8,
}

impl DefaultSymbolTable {
    pub fn new(handle_gen: Arc<RwLock<HandleGenerator>>, next_frame_offset: i8) -> DefaultSymbolTable {
        DefaultSymbolTable {
            handle_gen: handle_gen,
            symbols: SymbolMap::new(),
            next_frame_offset: next_frame_offset,
        }
    }
}

impl SymbolTable for DefaultSymbolTable {
    fn new_block_name(&mut self) -> (SymbolName, SymbolRef) {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        let symbol_name = Arc::new(format!("__L{:06X}_", symbol_ref));
        self.symbols
            .insert(SymbolName::clone(&symbol_name), symbol_ref, Symbol::Block);
        (symbol_name, symbol_ref)
    }

    fn insert_block(&mut self, symbol_name: SymbolName) -> Option<SymbolRef> {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols.insert(symbol_name, symbol_ref, Symbol::Block)
    }

    fn has_symbol(&self, symbol_name: &SymbolName) -> bool {
        self.symbols.find_by_name(symbol_name).is_some()
    }

    fn find_symbol(&self, symbol_name: &SymbolName) -> Option<SymbolRef> {
        self.symbols.get_ref(symbol_name)
    }

    fn get_symbol_name(&self, symbol_ref: SymbolRef) -> Option<SymbolName> {
        self.symbols.get_name(symbol_ref)
    }

    fn next_frame_offset(&mut self, local_size: usize) -> i8 {
        let result = self.next_frame_offset;
        self.next_frame_offset += local_size as i8;
        result
    }

    fn create_temporary(&mut self, typ: Type) -> SymbolRef {
        let next_location = self.next_frame_offset(typ.size());
        let symbol_name = SymbolName::new(format!("tmp#{}", next_location));
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols.insert(
            SymbolName::clone(&symbol_name),
            symbol_ref,
            Symbol::Variable(Variable::new(typ, Location::FrameOffset(next_location))),
        );
        symbol_ref
    }

    fn create_temporary_location(&mut self, typ: Type) -> Location {
        let symbol = self.create_temporary(typ);
        self.variable(symbol).unwrap().location
    }

    fn insert_constant(&mut self, symbol_name: SymbolName, value: TypedValue) -> Option<SymbolRef> {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols
            .insert(symbol_name, symbol_ref, Symbol::Constant(value))
    }

    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<TypedValue> {
        if let Some(&Symbol::Constant(val)) = self.symbols.find_by_name(symbol_name) {
            Some(val)
        } else {
            None
        }
    }

    fn constant(&self, symbol_ref: SymbolRef) -> Option<TypedValue> {
        if let Some(&Symbol::Constant(val)) = self.symbols.find_by_ref(symbol_ref) {
            Some(val)
        } else {
            None
        }
    }

    fn insert_function(&mut self, symbol_name: SymbolName, metadata: FunctionMetadataPtr) -> Option<SymbolRef> {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols
            .insert(symbol_name, symbol_ref, Symbol::Function(metadata))
    }

    fn function_by_name(&self, symbol_name: &SymbolName) -> Option<FunctionMetadataPtr> {
        if let Some(&Symbol::Function(ref func)) = self.symbols.find_by_name(symbol_name) {
            Some(FunctionMetadataPtr::clone(func))
        } else {
            None
        }
    }

    fn function(&self, symbol_ref: SymbolRef) -> Option<FunctionMetadataPtr> {
        if let Some(&Symbol::Function(ref func)) = self.symbols.find_by_ref(symbol_ref) {
            Some(FunctionMetadataPtr::clone(func))
        } else {
            None
        }
    }

    fn insert_variable(&mut self, symbol_name: SymbolName, variable: Variable) -> Option<SymbolRef> {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols
            .insert(symbol_name, symbol_ref, Symbol::Variable(variable))
    }

    fn variable_by_name(&self, symbol_name: &SymbolName) -> Option<Variable> {
        if let Some(&Symbol::Variable(v)) = self.symbols.find_by_name(symbol_name) {
            Some(v)
        } else {
            None
        }
    }

    fn variable(&self, symbol_ref: SymbolRef) -> Option<Variable> {
        if let Some(&Symbol::Variable(v)) = self.symbols.find_by_ref(symbol_ref) {
            Some(v)
        } else {
            None
        }
    }

    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a> {
        self.symbols.variables()
    }

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<Type> {
        if let Some(symbol) = self.symbols.find_by_ref(symbol_ref) {
            match *symbol {
                Symbol::Constant(ref data) => Some(data.get_type()),
                Symbol::Variable(ref data) => Some(data.type_name),
                Symbol::Function(ref data) => Some(data.read().unwrap().return_type),
                Symbol::Block => None,
            }
        } else {
            None
        }
    }

    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<Type> {
        match self.symbols.get_ref(symbol_name) {
            Some(symbol_ref) => self.type_of(symbol_ref),
            None => None,
        }
    }

    fn handle_gen(&self) -> Arc<RwLock<HandleGenerator>> {
        Arc::clone(&self.handle_gen)
    }
}

#[derive(Debug)]
pub struct ParentedSymbolTableWrapper {
    parent: Arc<RwLock<SymbolTable>>,
    child: Box<SymbolTable>,
}

impl ParentedSymbolTableWrapper {
    pub fn new(parent: Arc<RwLock<SymbolTable>>, child: Box<SymbolTable>) -> ParentedSymbolTableWrapper {
        ParentedSymbolTableWrapper {
            parent: parent,
            child: child,
        }
    }
}

impl SymbolTable for ParentedSymbolTableWrapper {
    fn new_block_name(&mut self) -> (SymbolName, SymbolRef) {
        self.parent.write().unwrap().new_block_name()
    }

    fn insert_block(&mut self, symbol_name: SymbolName) -> Option<SymbolRef> {
        self.parent.write().unwrap().insert_block(symbol_name)
    }

    fn has_symbol(&self, symbol_name: &SymbolName) -> bool {
        self.parent.read().unwrap().has_symbol(symbol_name) || self.child.has_symbol(symbol_name)
    }

    fn find_symbol(&self, symbol_name: &SymbolName) -> Option<SymbolRef> {
        if let Some(symbol_ref) = self.child.find_symbol(symbol_name) {
            Some(symbol_ref)
        } else {
            self.parent.read().unwrap().find_symbol(symbol_name)
        }
    }

    fn get_symbol_name(&self, symbol_ref: SymbolRef) -> Option<SymbolName> {
        if let Some(symbol_name) = self.child.get_symbol_name(symbol_ref) {
            Some(symbol_name)
        } else {
            self.parent.read().unwrap().get_symbol_name(symbol_ref)
        }
    }

    fn next_frame_offset(&mut self, local_size: usize) -> i8 {
        self.child.next_frame_offset(local_size)
    }

    fn create_temporary(&mut self, typ: Type) -> SymbolRef {
        self.child.create_temporary(typ)
    }

    fn create_temporary_location(&mut self, typ: Type) -> Location {
        self.child.create_temporary_location(typ)
    }

    fn insert_constant(&mut self, symbol_name: SymbolName, value: TypedValue) -> Option<SymbolRef> {
        self.child.insert_constant(symbol_name, value)
    }

    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<TypedValue> {
        if let Some(constant) = self.child.constant_by_name(symbol_name) {
            Some(constant)
        } else {
            self.parent.read().unwrap().constant_by_name(symbol_name)
        }
    }

    fn constant(&self, symbol_ref: SymbolRef) -> Option<TypedValue> {
        if let Some(constant) = self.child.constant(symbol_ref) {
            Some(constant)
        } else {
            self.parent.read().unwrap().constant(symbol_ref)
        }
    }

    fn insert_function(&mut self, symbol_name: SymbolName, metadata: FunctionMetadataPtr) -> Option<SymbolRef> {
        self.child.insert_function(symbol_name, metadata)
    }

    fn function_by_name(&self, symbol_name: &SymbolName) -> Option<FunctionMetadataPtr> {
        if let Some(function) = self.child.function_by_name(symbol_name) {
            Some(function)
        } else {
            self.parent.read().unwrap().function_by_name(symbol_name)
        }
    }

    fn function(&self, symbol_ref: SymbolRef) -> Option<FunctionMetadataPtr> {
        if let Some(function) = self.child.function(symbol_ref) {
            Some(function)
        } else {
            self.parent.read().unwrap().function(symbol_ref)
        }
    }

    fn insert_variable(&mut self, symbol_name: SymbolName, variable: Variable) -> Option<SymbolRef> {
        self.child.insert_variable(symbol_name, variable)
    }

    fn variable_by_name(&self, symbol_name: &SymbolName) -> Option<Variable> {
        if let Some(variable) = self.child.variable_by_name(symbol_name) {
            Some(variable)
        } else {
            self.parent.read().unwrap().variable_by_name(symbol_name)
        }
    }

    fn variable(&self, symbol_ref: SymbolRef) -> Option<Variable> {
        if let Some(variable) = self.child.variable(symbol_ref) {
            Some(variable)
        } else {
            self.parent.read().unwrap().variable(symbol_ref)
        }
    }

    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a> {
        self.child.variables()
    }

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<Type> {
        if let Some(typ) = self.child.type_of(symbol_ref) {
            Some(typ)
        } else {
            self.parent.read().unwrap().type_of(symbol_ref)
        }
    }

    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<Type> {
        if let Some(typ) = self.child.type_of_by_name(symbol_name) {
            Some(typ)
        } else {
            self.parent.read().unwrap().type_of_by_name(symbol_name)
        }
    }

    fn handle_gen(&self) -> Arc<RwLock<HandleGenerator>> {
        self.parent.read().unwrap().handle_gen()
    }
}
