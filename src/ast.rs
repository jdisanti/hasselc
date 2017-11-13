use std::sync::Arc;
use lalrpop_util;
use error;

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
    pub fn parse<'a>(text: &'a str) -> error::Result<Vec<Expression>> {
        let mut errors: Vec<lalrpop_util::ErrorRecovery<usize, (usize, &'a str), ()>> = Vec::new();
        let ast = ::grammar::parse_Program(&mut errors, text);
        if errors.is_empty() {
            match ast {
                Ok(expression) => Ok(expression),
                Err(err) => Err(translate_errors(text, [err].iter()).into()),
            }
        } else {
            Err(translate_errors(text, errors.iter().map(|err| &err.error)).into())
        }
    }
}

fn offset_to_row_col(program: &str, offset: usize) -> (usize, usize) {
    let mut row: usize = 1;
    let mut col: usize = 1;

    for i in 0..offset {
        if &program[i..i + 1] == "\n" {
            row += 1;
            col = 1;
        } else {
            col += 1;
        }
    }

    (row, col)
}

fn translate_errors<'a, I>(program: &str, errors: I) -> error::ErrorKind
        where I: Iterator<Item = &'a lalrpop_util::ParseError<usize, (usize, &'a str), ()>> {
    let mut messages = Vec::new();
    for error in errors {
        match *error {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let (row, col) = offset_to_row_col(program, location);
                messages.push(format!("{}:{}: invalid token", row, col));
            }
            lalrpop_util::ParseError::UnrecognizedToken { ref token, ref expected } => match *token {
                Some((start, token, _end)) => {
                    let (row, col) = offset_to_row_col(program, start);
                    messages.push(format!(
                        "{}:{}: unexpected token \"{}\". Expected one of: {:?}",
                        row,
                        col,
                        token.1,
                        expected
                    ));
                }
                None => {
                    messages.push(format!("unexpected EOF"));
                }
            },
            lalrpop_util::ParseError::ExtraToken { ref token } => {
                let (row, col) = offset_to_row_col(program, token.0);
                messages.push(format!("{}:{}: extra token \"{}\"", row, col, (token.1).1));
            }
            lalrpop_util::ParseError::User { ref error } => {
                messages.push(format!("{:?}", error));
            }
        }
    }
    return error::ErrorKind::ParseError(messages).into();
}