use std::sync::{Arc, RwLock};
use ast::BinaryOperator;
use symbol_table::{FunctionMetadataPtr, Location, SymbolRef, SymbolTable};
use src_tag::SrcTag;

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
pub enum Statement {
    Assign { symbol: SymbolRef, value: Expr },
    Break,
    Call(Expr),
    Conditional(ConditionalData),
    LeftShift(SymbolRef),
    RotateLeft(SymbolRef),
    RotateRight(SymbolRef),
    Return(Option<Expr>),
    GoTo(Arc<String>),
}

// Intermediate representation
#[derive(Debug, Clone)]
pub enum IR {
    AnonymousBlock {
        symbol_table: Arc<RwLock<SymbolTable>>,
        location: Option<Location>,
        body: Vec<Statement>,
    },
    FunctionBlock {
        location: Option<Location>,
        local_symbols: Arc<RwLock<SymbolTable>>,
        body: Vec<Statement>,
        metadata: FunctionMetadataPtr,
    },
}

impl IR {
    pub fn new_anonymous_block(global_symbol_table: Arc<RwLock<SymbolTable>>) -> IR {
        IR::AnonymousBlock {
            symbol_table: global_symbol_table,
            location: None,
            body: Vec::new(),
        }
    }

    pub fn new_function_block(
        parent_symbol_table: Arc<RwLock<SymbolTable>>,
        location: Option<Location>,
        metadata: FunctionMetadataPtr,
    ) -> IR {
        let frame_size = metadata.read().unwrap().parameters.iter()
            .map(|p| p.type_name.size())
            .fold(0, |acc, size| acc + size);
        let mut symbol_table = SymbolTable::new_from_parent(parent_symbol_table, frame_size as i8);

        let mut frame_offset = 0i8;
        for parameter in &metadata.read().unwrap().parameters {
            symbol_table.variables.insert(
                SymbolRef(parameter.name.clone()),
                (parameter.type_name, Location::FrameOffset(frame_offset)),
            );
            frame_offset += parameter.type_name.size() as i8;
        }

        IR::FunctionBlock {
            location: location,
            local_symbols: Arc::new(RwLock::new(symbol_table)),
            body: Vec::new(),
            metadata: metadata,
        }
    }

    pub fn is_empty_anonymous(&self) -> bool {
        match self {
            &IR::AnonymousBlock { ref body, .. } => body.is_empty(),
            &IR::FunctionBlock { .. } => false,
        }
    }

    pub fn symbol_table(&mut self) -> Arc<RwLock<SymbolTable>> {
        match self {
            &mut IR::AnonymousBlock { .. } => unreachable!(),
            &mut IR::FunctionBlock {
                ref mut local_symbols,
                ..
            } => local_symbols.clone(),
        }
    }

    pub fn location(&self) -> &Option<Location> {
        match *self {
            IR::AnonymousBlock { ref location, .. } => location,
            IR::FunctionBlock { ref location, .. } => location,
        }
    }

    pub fn set_location(&mut self, new_location: Location) {
        match self {
            &mut IR::AnonymousBlock {
                ref mut location, ..
            } => {
                location.get_or_insert(new_location);
            }
            &mut IR::FunctionBlock {
                ref mut location, ..
            } => {
                location.get_or_insert(new_location);
            }
        }
    }

    pub fn body_mut<'a>(&'a mut self) -> &'a mut Vec<Statement> {
        match self {
            &mut IR::AnonymousBlock { ref mut body, .. } => body,
            &mut IR::FunctionBlock { ref mut body, .. } => body,
        }
    }
}
