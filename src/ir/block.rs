//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use std::sync::{Arc, RwLock};
use error::{self, ErrorKind};
use parse::ast::BinaryOperator;
use src_tag::{SrcTag, SrcTagged};
use symbol_table::{DefaultSymbolTable, FunctionMetadata, FunctionMetadataPtr, Location, ParentedSymbolTableWrapper,
                   SymbolName, SymbolRef, SymbolTable, Variable};
use base_type::BaseType;

#[derive(Debug, new)]
pub struct ArrayIndexData {
    pub tag: SrcTag,
    pub array: SymbolRef,
    pub index: Box<Expr>,
    pub array_type: Option<BaseType>,
}

#[derive(Debug, new)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub op: BinaryOperator,
    pub result_type: Option<BaseType>,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, new)]
pub struct CallData {
    pub tag: SrcTag,
    pub function: SymbolName,
    pub arguments: Vec<Expr>,
    pub return_type: Option<BaseType>,
}

#[derive(Debug, new)]
pub struct NumberData {
    pub tag: SrcTag,
    pub value: i32,
    pub value_type: Option<BaseType>,
}

#[derive(Debug, new)]
pub struct SymbolData {
    pub tag: SrcTag,
    pub symbol: SymbolRef,
    pub value_type: Option<BaseType>,
}

#[derive(Debug)]
pub enum Expr {
    Number(NumberData),
    Symbol(SymbolData),
    BinaryOp(BinaryOpData),
    Call(CallData),
    ArrayIndex(ArrayIndexData),
}

impl SrcTagged for Expr {
    fn src_tag(&self) -> SrcTag {
        use self::Expr::*;
        match *self {
            Number(ref d) => d.tag,
            Symbol(ref d) => d.tag,
            BinaryOp(ref d) => d.tag,
            Call(ref d) => d.tag,
            ArrayIndex(ref d) => d.tag,
        }
    }
}

#[derive(Debug, new)]
pub struct AssignData {
    pub tag: SrcTag,
    pub value_type: Option<BaseType>,
    pub left_value: Expr,
    pub right_value: Expr,
}

#[derive(Debug, new)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub when_true: Vec<Statement>,
    pub when_false: Vec<Statement>,
}

#[derive(Debug, new)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: Arc<String>,
}

#[derive(Debug, new)]
pub struct InlineAsmData {
    pub tag: SrcTag,
    pub asm: Arc<String>,
}

#[derive(Debug, new)]
pub struct ReturnData {
    pub tag: SrcTag,
    pub value_type: Option<BaseType>,
    pub value: Option<Expr>,
}

#[derive(Debug, new)]
pub struct WhileLoopData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub body: Vec<Statement>,
}

#[derive(Debug)]
pub enum Statement {
    Assign(AssignData),
    Break,
    Call(CallData),
    Conditional(ConditionalData),
    GoTo(GoToData),
    InlineAsm(InlineAsmData),
    Return(ReturnData),
    WhileLoop(WhileLoopData),
}

// Intermediate representation
#[derive(Debug)]
pub struct Block {
    pub name: SymbolName,
    pub symbol: SymbolRef,
    pub location: Option<Location>,
    pub body: Vec<Statement>,
    pub symbol_table: Arc<RwLock<SymbolTable>>,
    pub metadata: FunctionMetadataPtr,
    pub anonymous: bool,
}

impl Block {
    pub fn new_anonymous(global_symbol_table: Arc<RwLock<SymbolTable>>) -> Block {
        let (symbol_name, symbol_ref) = global_symbol_table.write().unwrap().new_block_name();
        Block {
            name: SymbolName::clone(&symbol_name),
            symbol: symbol_ref,
            location: None,
            body: Vec::new(),
            symbol_table: global_symbol_table,
            metadata: Arc::new(RwLock::new(FunctionMetadata {
                name: symbol_name,
                location: None,
                parameters: Vec::new(),
                return_type: BaseType::Void,
                frame_size: 0,
            })),
            anonymous: true,
        }
    }

    pub fn new_named(
        src_tag: SrcTag,
        function_ref: SymbolRef,
        parent_symbol_table: Arc<RwLock<SymbolTable>>,
        location: Option<Location>,
        metadata: FunctionMetadataPtr,
    ) -> error::Result<Block> {
        let symbol_name = SymbolName::clone(&metadata.read().unwrap().name);
        let frame_size = metadata
            .read()
            .unwrap()
            .parameters
            .iter()
            .map(|p| p.base_type.size().unwrap())
            .fold(0, |acc, size| acc + size);
        let handle_gen = parent_symbol_table.read().unwrap().handle_gen();
        let mut symbol_table = ParentedSymbolTableWrapper::new(
            parent_symbol_table,
            Box::new(DefaultSymbolTable::new(handle_gen, frame_size as i8)),
        );

        let mut frame_offset = 0i8;
        for parameter in &metadata.read().unwrap().parameters {
            let name = SymbolName::clone(&parameter.name);
            let variable = Variable::new(
                parameter.base_type.clone(),
                Location::FrameOffset(frame_offset),
            );
            if symbol_table.insert_variable(SymbolName::clone(&name), variable).is_none() {
                return Err(ErrorKind::DuplicateSymbol(src_tag, name).into());
            }
            frame_offset += parameter.base_type.size().unwrap() as i8;
        }

        Ok(Block {
            name: symbol_name,
            symbol: function_ref,
            location: location,
            symbol_table: Arc::new(RwLock::new(symbol_table)),
            body: Vec::new(),
            metadata: metadata,
            anonymous: false,
        })
    }

    pub fn is_empty_anonymous(&self) -> bool {
        self.anonymous && self.body.is_empty()
    }
}
