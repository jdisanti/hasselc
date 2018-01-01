//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use parse::ast::NameType;
use base_type::BaseType;
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
    pub return_type: BaseType,
    pub frame_size: i8,
}

pub type FunctionMetadataPtr = Arc<RwLock<FunctionMetadata>>;

#[derive(Debug, Clone, new)]
pub struct Variable {
    pub base_type: BaseType,
    pub location: Location,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ConstantValue {
    Number(i32),
    Bytes(Arc<Vec<u8>>),
}

impl ConstantValue {
    pub fn number(&self) -> i32 {
        match *self {
            ConstantValue::Number(num) => num,
            _ => panic!("attempt to take a number from non-number constant value"),
        }
    }
}

#[derive(Debug, Clone, new)]
pub struct Constant {
    pub base_type: BaseType,
    pub value: ConstantValue,
}

#[derive(Clone, Debug)]
enum Symbol {
    Constant(Constant),
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

    fn data_constants<'a>(&'a self) -> Box<Iterator<Item = (SymbolRef, Arc<Vec<u8>>)> + 'a> {
        Box::new(
            self.by_ref
                .iter()
                .filter_map(|(symbol_ref, symbol)| match *symbol {
                    Symbol::Constant(ref constant) => match constant.value {
                        ConstantValue::Bytes(ref bytes) => Some((*symbol_ref, Arc::clone(bytes))),
                        _ => None,
                    },
                    _ => None,
                }),
        )
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

    fn create_temporary(&mut self, base_type: &BaseType) -> SymbolRef;
    fn create_temporary_location(&mut self, base_type: &BaseType) -> Location;

    fn insert_constant(
        &mut self,
        symbol_name: SymbolName,
        base_type: &BaseType,
        value: ConstantValue,
    ) -> Option<SymbolRef>;
    fn insert_unnamed_constant(&mut self, base_type: &BaseType, value: ConstantValue) -> Option<SymbolRef>;
    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<Constant>;
    fn constant(&self, symbol_ref: SymbolRef) -> Option<Constant>;

    fn insert_function(&mut self, symbol_name: SymbolName, metadata: FunctionMetadataPtr) -> Option<SymbolRef>;
    fn function_by_name(&self, symbol_name: &SymbolName) -> Option<FunctionMetadataPtr>;
    fn function(&self, symbol_ref: SymbolRef) -> Option<FunctionMetadataPtr>;

    fn insert_variable(&mut self, symbol_name: SymbolName, variable: Variable) -> Option<SymbolRef>;
    fn variable_by_name(&self, symbol_name: &SymbolName) -> Option<Variable>;
    fn variable(&self, symbol_ref: SymbolRef) -> Option<Variable>;
    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a>;

    fn data_constants<'a>(&'a self) -> Box<Iterator<Item = (SymbolRef, Arc<Vec<u8>>)> + 'a>;

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<BaseType>;
    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<BaseType>;

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

    fn create_temporary(&mut self, base_type: &BaseType) -> SymbolRef {
        let next_location = self.next_frame_offset(base_type.size().unwrap());
        let symbol_name = SymbolName::new(format!("tmp#{}", next_location));
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols.insert(
            SymbolName::clone(&symbol_name),
            symbol_ref,
            Symbol::Variable(Variable::new(
                base_type.clone(),
                Location::FrameOffset(next_location),
            )),
        );
        symbol_ref
    }

    fn create_temporary_location(&mut self, base_type: &BaseType) -> Location {
        let symbol = self.create_temporary(base_type);
        self.variable(symbol).unwrap().location
    }

    fn insert_constant(
        &mut self,
        symbol_name: SymbolName,
        base_type: &BaseType,
        value: ConstantValue,
    ) -> Option<SymbolRef> {
        let symbol_ref = self.handle_gen.write().unwrap().new_handle();
        self.symbols.insert(
            symbol_name,
            symbol_ref,
            Symbol::Constant(Constant::new(base_type.clone(), value)),
        )
    }

    fn insert_unnamed_constant(&mut self, base_type: &BaseType, value: ConstantValue) -> Option<SymbolRef> {
        let name = Arc::new(format!(
            "__UC{:06X}_",
            self.handle_gen.write().unwrap().new_handle()
        ));
        self.insert_constant(name, base_type, value)
    }

    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<Constant> {
        if let Some(&Symbol::Constant(ref val)) = self.symbols.find_by_name(symbol_name) {
            Some(val.clone())
        } else {
            None
        }
    }

    fn constant(&self, symbol_ref: SymbolRef) -> Option<Constant> {
        if let Some(&Symbol::Constant(ref val)) = self.symbols.find_by_ref(symbol_ref) {
            Some(val.clone())
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
        if let Some(&Symbol::Variable(ref v)) = self.symbols.find_by_name(symbol_name) {
            Some(v.clone())
        } else {
            None
        }
    }

    fn variable(&self, symbol_ref: SymbolRef) -> Option<Variable> {
        if let Some(&Symbol::Variable(ref v)) = self.symbols.find_by_ref(symbol_ref) {
            Some(v.clone())
        } else {
            None
        }
    }

    fn variables<'a>(&'a self) -> Box<Iterator<Item = &'a Variable> + 'a> {
        self.symbols.variables()
    }

    fn data_constants<'a>(&'a self) -> Box<Iterator<Item = (SymbolRef, Arc<Vec<u8>>)> + 'a> {
        self.symbols.data_constants()
    }

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<BaseType> {
        if let Some(symbol) = self.symbols.find_by_ref(symbol_ref) {
            match *symbol {
                Symbol::Constant(ref data) => Some(data.base_type.clone()),
                Symbol::Variable(ref data) => Some(data.base_type.clone()),
                Symbol::Function(ref data) => Some(data.read().unwrap().return_type.clone()),
                Symbol::Block => None,
            }
        } else {
            None
        }
    }

    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<BaseType> {
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

    fn create_temporary(&mut self, base_type: &BaseType) -> SymbolRef {
        self.child.create_temporary(base_type)
    }

    fn create_temporary_location(&mut self, base_type: &BaseType) -> Location {
        self.child.create_temporary_location(base_type)
    }

    fn insert_constant(
        &mut self,
        symbol_name: SymbolName,
        base_type: &BaseType,
        value: ConstantValue,
    ) -> Option<SymbolRef> {
        self.child.insert_constant(symbol_name, base_type, value)
    }

    fn insert_unnamed_constant(&mut self, base_type: &BaseType, value: ConstantValue) -> Option<SymbolRef> {
        self.child.insert_unnamed_constant(base_type, value)
    }

    fn constant_by_name(&self, symbol_name: &SymbolName) -> Option<Constant> {
        if let Some(constant) = self.child.constant_by_name(symbol_name) {
            Some(constant)
        } else {
            self.parent.read().unwrap().constant_by_name(symbol_name)
        }
    }

    fn constant(&self, symbol_ref: SymbolRef) -> Option<Constant> {
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

    fn data_constants<'a>(&'a self) -> Box<Iterator<Item = (SymbolRef, Arc<Vec<u8>>)> + 'a> {
        self.child.data_constants()
    }

    fn type_of(&self, symbol_ref: SymbolRef) -> Option<BaseType> {
        if let Some(typ) = self.child.type_of(symbol_ref) {
            Some(typ)
        } else {
            self.parent.read().unwrap().type_of(symbol_ref)
        }
    }

    fn type_of_by_name(&self, symbol_name: &SymbolName) -> Option<BaseType> {
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
