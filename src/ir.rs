use std::sync::{Arc, RwLock};
use ast::{BinaryOperator, Type};
use symbol_table::{FunctionMetadata, FunctionMetadataPtr, Location, SymbolRef, SymbolTable, Variable};
use src_tag::SrcTag;
use error::{self, ErrorKind};

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub when_true: Vec<Statement>,
    pub when_false: Vec<Statement>,
}

impl ConditionalData {
    pub fn new(tag: SrcTag, condition: Expr, when_true: Vec<Statement>, when_false: Vec<Statement>) -> ConditionalData {
        ConditionalData {
            tag: tag,
            condition: condition,
            when_true: when_true,
            when_false: when_false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WhileLoopData {
    pub tag: SrcTag,
    pub condition: Expr,
    pub body: Vec<Statement>,
}

impl WhileLoopData {
    pub fn new(tag: SrcTag, condition: Expr, body: Vec<Statement>) -> WhileLoopData {
        WhileLoopData {
            tag: tag,
            condition: condition,
            body: body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssignData {
    pub tag: SrcTag,
    pub symbol: SymbolRef,
    pub value: Expr,
}

impl AssignData {
    pub fn new(tag: SrcTag, symbol: SymbolRef, value: Expr) -> AssignData {
        AssignData {
            tag: tag,
            symbol: symbol,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallData {
    pub tag: SrcTag,
    pub call_expression: Expr,
}

impl CallData {
    pub fn new(tag: SrcTag, call_expression: Expr) -> CallData {
        CallData {
            tag: tag,
            call_expression: call_expression,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReturnData {
    pub tag: SrcTag,
    pub value: Option<Expr>,
}

impl ReturnData {
    pub fn new(tag: SrcTag, value: Option<Expr>) -> ReturnData {
        ReturnData {
            tag: tag,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: Arc<String>,
}

impl GoToData {
    pub fn new(tag: SrcTag, destination: Arc<String>) -> GoToData {
        GoToData {
            tag: tag,
            destination: destination,
        }
    }
}

#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
