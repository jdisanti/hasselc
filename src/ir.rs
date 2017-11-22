use std::sync::{Arc, RwLock};
use ast::{BinaryOperator, Type};
use symbol_table::{FunctionMetadata, FunctionMetadataPtr, Location, SymbolRef, SymbolTable, Variable};
use src_tag::SrcTag;
use error::{self, ErrorKind};

#[derive(Debug, new)]
pub struct NumberData {
    pub tag: SrcTag,
    pub value: i32,
}

#[derive(Debug, new)]
pub struct SymbolData {
    pub tag: SrcTag,
    pub name: SymbolRef,
}

#[derive(Debug, new)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub op: BinaryOperator,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, new)]
pub struct CallData {
    pub tag: SrcTag,
    pub function: SymbolRef,
    pub arguments: Vec<Expr>,
}

#[derive(Debug)]
pub enum Expr {
    Number(NumberData),
    Symbol(SymbolData),
    BinaryOp(BinaryOpData),
    Call(CallData),
}

#[derive(Debug, new)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub when_true: Vec<Statement>,
    pub when_false: Vec<Statement>,
}

#[derive(Debug, new)]
pub struct WhileLoopData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub body: Vec<Statement>,
}

#[derive(Debug, new)]
pub struct AssignData {
    pub tag: SrcTag,
    pub symbol: SymbolRef,
    pub value: Expr,
}

#[derive(Debug, new)]
pub struct ReturnData {
    pub tag: SrcTag,
    pub value: Option<Expr>,
}

#[derive(Debug, new)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: Arc<String>,
}

#[derive(Debug)]
pub enum Statement {
    Assign(AssignData),
    Break,
    Call(CallData),
    Conditional(ConditionalData),
    Return(ReturnData),
    GoTo(GoToData),
    WhileLoop(WhileLoopData),
}

// Intermediate representation
#[derive(Debug)]
pub struct Block {
    pub name: Arc<String>,
    pub location: Option<Location>,
    pub body: Vec<Statement>,
    pub symbol_table: Arc<RwLock<SymbolTable>>,
    pub metadata: FunctionMetadataPtr,
    pub anonymous: bool,
}

impl Block {
    pub fn new_anonymous(global_symbol_table: Arc<RwLock<SymbolTable>>) -> Block {
        let name = global_symbol_table.write().unwrap().new_block_name();
        Block {
            name: Arc::clone(&name),
            location: None,
            body: Vec::new(),
            symbol_table: global_symbol_table,
            metadata: Arc::new(RwLock::new(FunctionMetadata {
                name: name,
                location: None,
                parameters: Vec::new(),
                return_type: Type::Void,
                frame_size: 0,
            })),
            anonymous: true,
        }
    }

    pub fn new_named(
        src_tag: SrcTag,
        parent_symbol_table: Arc<RwLock<SymbolTable>>,
        location: Option<Location>,
        metadata: FunctionMetadataPtr,
    ) -> error::Result<Block> {
        let frame_size = metadata
            .read()
            .unwrap()
            .parameters
            .iter()
            .map(|p| p.type_name.size())
            .fold(0, |acc, size| acc + size);
        let mut symbol_table = SymbolTable::new_from_parent(parent_symbol_table, frame_size as i8);

        let mut frame_offset = 0i8;
        for parameter in &metadata.read().unwrap().parameters {
            let name = SymbolRef::clone(&parameter.name);
            let variable = Variable::new(parameter.type_name, Location::FrameOffset(frame_offset));
            if !symbol_table.insert_variable(SymbolRef::clone(&name), variable) {
                return Err(ErrorKind::DuplicateSymbol(src_tag, name).into());
            }
            frame_offset += parameter.type_name.size() as i8;
        }

        let name = Arc::clone(&metadata.read().unwrap().name);
        Ok(Block {
            name: name,
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
