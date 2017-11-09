#[derive(Debug)]
pub enum Keyword {
    Break,
    Const,
    Def,
    End,
    For,
    If,
    In,
    LeftShift,
    Org,
    Return,
    RotateLeft,
    RotateRight,
    To,
}

#[derive(Debug)]
pub enum Type {
    Ptr,
    U8,
    U16,
    Void,
}

#[derive(Debug)]
pub struct NameType(pub String, pub Type);

#[derive(Debug)]
pub enum Operator {
    Add,
    Assign,
    EqualTo,
    GreaterThan,
    GreaterThanEqualTo,
    LessThan,
    LessThanEqualTo,
    Subtract,
}

#[derive(Debug)]
pub enum Constant {
    Integer(i32),
    Text(String),
}

#[derive(Debug)]
pub enum Expr {
    Void,
    Name(String),
    ConstantValue(Constant),
    Command(Keyword),
    CommandOnName(Keyword, String),
    Return(Box<Expr>),
    DeclareConst {
        name: String,
        value: Constant
    },
    DeclareFunction {
        name: String,
        parameters: Vec<NameType>,
        return_type: Type,
    },
    Function {
        declaration: Box<Expr>,
        body: Vec<Expr>,
    },
    ForLoop {
        index: NameType,
        start: Constant,
        finish: Constant,
        body: Vec<Expr>,
    },
    Org(i32),
}

#[derive(Debug)]
pub struct Program(pub Vec<Expr>);