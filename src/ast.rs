use std::sync::Arc;

#[derive(Debug, Clone)]
pub enum Literal {
    Int(i32),
    Str(Arc<String>),
}

#[derive(Debug, Copy, Clone)]
pub enum Type {
    U8,
    U16,
    Void,
}

impl Type {
    pub fn size(&self) -> usize {
        match *self {
            Type::U8 => 1,
            Type::U16 => 2,
            Type::Void => 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameType {
    pub name: Arc<String>,
    pub type_name: Type,
}

#[derive(Debug, Copy, Clone)]
pub enum BinaryOperator {
    Add,
    Sub,
    Mul,
    Div,
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    Equal,
    NotEqual,
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(i32),
    Name(Arc<String>),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assignment {
        name: Arc<String>,
        value: Box<Expression>,
    },
    CallFunction {
        name: Arc<String>,
        arguments: Vec<Expression>,
    },
    DeclareConst {
        name_type: NameType,
        value: Box<Expression>,
    },
    DeclareFunction {
        name: Arc<String>,
        parameters: Vec<NameType>,
        return_type: Type,
        body: Vec<Expression>,
    },
    DeclareVariable {
        name_type: NameType,
        value: Box<Expression>,
    },
    DeclareRegister { name_type: NameType, location: i32 },
    LeftShift(Arc<String>),
    RotateLeft(Arc<String>),
    RotateRight(Arc<String>),
    Org { org: i32 },
    Break,
    Return { value: Box<Expression> },
    GoTo(Arc<String>),
    Comment,
    Error,
}

type SyntaxError<'input> = ::lalrpop_util::ErrorRecovery<usize, (usize, &'input str), ()>;

impl Expression {
    pub fn parse<'a>(text: &'a str) -> Result<Vec<Expression>, Vec<SyntaxError<'a>>> {
        let mut errors = Vec::new();
        let ast = ::grammar::parse_Program(&mut errors, text);
        if errors.is_empty() {
            Ok(ast.unwrap())
        } else {
            Err(errors)
        }
    }
}
