#[derive(Debug)]
pub enum Literal {
    Int(i32),
    Str(String),
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

    pub fn is_void(&self) -> bool {
        match *self {
            Type::Void => true,
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameType {
    pub name: String,
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
    DeclareRegister {
        name_type: NameType,
        location: i32,
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