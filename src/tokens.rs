#[derive(Debug)]
pub enum Literal {
    Int(i32),
    Str(String),
}

#[derive(Debug)]
pub enum Type {
    U8,
    U16,
    Void,
}

#[derive(Debug)]
pub struct NameType {
    pub name: String,
    pub type_name: Type,
}

#[derive(Debug)]
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

#[derive(Debug)]
pub enum Expression {
    Number(i32),
    Name(String),
    BinaryOp {
        op: BinaryOperator,
        left: Box<Expression>,
        right: Box<Expression>,
    },
    Assignment {
        name: String,
        value: Box<Expression>,
    },
    CallFunction {
        name: String,
        arguments: Vec<Expression>,
    },
    DeclareConst {
        name_type: NameType,
        value: Box<Expression>,
    },
    DeclareFunction {
        name: String,
        parameters: Vec<NameType>,
        return_type: Type,
        body: Vec<Expression>,
    },
    DeclareVariable {
        name_type: NameType,
        value: Box<Expression>,
    },
    LeftShift(String),
    RotateLeft(String),
    RotateRight(String),
    Org {
        org: i32,
    },
    Break,
    Return {
        value: Box<Expression>,
    },
}
