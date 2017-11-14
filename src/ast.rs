use std::sync::Arc;
use lalrpop_util;
use src_tag::SrcTag;
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
pub struct NumberData {
    pub tag: SrcTag,
    pub value: i32,
}

impl NumberData {
    pub fn new(tag: SrcTag, value: i32) -> NumberData {
        NumberData {
            tag: tag,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NameData {
    pub tag: SrcTag,
    pub name: Arc<String>,
}

impl NameData {
    pub fn new(tag: SrcTag, name: Arc<String>) -> NameData {
        NameData {
            tag: tag,
            name: name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub op: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

impl BinaryOpData {
    pub fn new(tag: SrcTag, op: BinaryOperator, left: Box<Expression>, right: Box<Expression>) -> BinaryOpData {
        BinaryOpData {
            tag: tag,
            op: op,
            left: left,
            right: right,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AssignmentData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub value: Box<Expression>,
}

impl AssignmentData {
    pub fn new(tag: SrcTag, name: Arc<String>, value: Box<Expression>) -> AssignmentData {
        AssignmentData {
            tag: tag,
            name: name,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CallFunctionData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub arguments: Vec<Expression>,
}

impl CallFunctionData {
    pub fn new(tag: SrcTag, name: Arc<String>, arguments: Vec<Expression>) -> CallFunctionData {
        CallFunctionData {
            tag: tag,
            name: name,
            arguments: arguments,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclareConstData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub value: Box<Expression>,
}

impl DeclareConstData {
    pub fn new(tag: SrcTag, name_type: NameType, value: Box<Expression>) -> DeclareConstData {
        DeclareConstData {
            tag: tag,
            name_type: name_type,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclareFunctionData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub parameters: Vec<NameType>,
    pub return_type: Type,
    pub body: Vec<Expression>,
}

impl DeclareFunctionData {
    pub fn new(
        tag: SrcTag,
        name: Arc<String>,
        parameters: Vec<NameType>,
        return_type: Type,
        body: Vec<Expression>,
    ) -> DeclareFunctionData {
        DeclareFunctionData {
            tag: tag,
            name: name,
            parameters: parameters,
            return_type: return_type,
            body: body,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclareVariableData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub value: Box<Expression>,
}

impl DeclareVariableData {
    pub fn new(tag: SrcTag, name_type: NameType, value: Box<Expression>) -> DeclareVariableData {
        DeclareVariableData {
            tag: tag,
            name_type: name_type,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeclareRegisterData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub location: i32,
}

impl DeclareRegisterData {
    pub fn new(tag: SrcTag, name_type: NameType, location: i32) -> DeclareRegisterData {
        DeclareRegisterData {
            tag: tag,
            name_type: name_type,
            location: location,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OrgData {
    pub tag: SrcTag,
    pub address: i32,
}

impl OrgData {
    pub fn new(tag: SrcTag, address: i32) -> OrgData {
        OrgData {
            tag: tag,
            address: address,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReturnData {
    pub tag: SrcTag,
    pub value: Option<Box<Expression>>,
}

impl ReturnData {
    pub fn new(tag: SrcTag, value: Option<Box<Expression>>) -> ReturnData {
        ReturnData {
            tag: tag,
            value: value,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Expression {
    Number(NumberData),
    Name(NameData),
    BinaryOp(BinaryOpData),
    Assignment(AssignmentData),
    CallFunction(CallFunctionData),
    DeclareConst(DeclareConstData),
    DeclareFunction(DeclareFunctionData),
    DeclareVariable(DeclareVariableData),
    DeclareRegister(DeclareRegisterData),
    LeftShift(Arc<String>),
    RotateLeft(Arc<String>),
    RotateRight(Arc<String>),
    Org(OrgData),
    Break,
    Return(ReturnData),
    GoTo(Arc<String>),
    Comment,
    Error,
}

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

fn translate_errors<'a, I>(program: &str, errors: I) -> error::ErrorKind
where
    I: Iterator<Item = &'a lalrpop_util::ParseError<usize, (usize, &'a str), ()>>,
{
    let mut messages = Vec::new();
    for error in errors {
        match *error {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let (row, col) = SrcTag(location).row_col(program);
                messages.push(format!("{}:{}: invalid token", row, col));
            }
            lalrpop_util::ParseError::UnrecognizedToken {
                ref token,
                ref expected,
            } => match *token {
                Some((start, token, _end)) => {
                    let (row, col) = SrcTag(start).row_col(program);
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
                let (row, col) = SrcTag(token.0).row_col(program);
                messages.push(format!("{}:{}: extra token \"{}\"", row, col, (token.1).1));
            }
            lalrpop_util::ParseError::User { ref error } => {
                messages.push(format!("{:?}", error));
            }
        }
    }
    return error::ErrorKind::ParseError(messages).into();
}
