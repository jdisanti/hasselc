use std::sync::Arc;
use lalrpop_util;
use src_tag::SrcTag;
use error;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Literal {
    Int(i32),
    Str(Arc<String>),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct NameType {
    pub name: Arc<String>,
    pub type_name: Type,
}

impl NameType {
    pub fn new(name: Arc<String>, type_name: Type) -> NameType {
        NameType {
            name: name,
            type_name: type_name,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
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

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Box<Expression>,
    pub when_true: Vec<Expression>,
    pub when_false: Vec<Expression>,
}

impl ConditionalData {
    pub fn new(
        tag: SrcTag,
        condition: Box<Expression>,
        when_true: Vec<Expression>,
        when_false: Vec<Expression>,
    ) -> ConditionalData {
        ConditionalData {
            tag: tag,
            condition: condition,
            when_true: when_true,
            when_false: when_false,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Expression {
    Assignment(AssignmentData),
    BinaryOp(BinaryOpData),
    Break,
    CallFunction(CallFunctionData),
    Comment,
    Conditional(ConditionalData),
    DeclareConst(DeclareConstData),
    DeclareFunction(DeclareFunctionData),
    DeclareRegister(DeclareRegisterData),
    DeclareVariable(DeclareVariableData),
    Error,
    GoTo(Arc<String>),
    LeftShift(Arc<String>),
    Name(NameData),
    Number(NumberData),
    Org(OrgData),
    Return(ReturnData),
    RotateLeft(Arc<String>),
    RotateRight(Arc<String>),
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn operator_precedence_multiplication() {
        let program = "a = 2 + 9 * 1;";
        let ast = Expression::parse(&program).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag(0),
                Arc::new("a".into()),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag(4),
                    BinaryOperator::Add,
                    Box::new(Expression::Number(NumberData::new(SrcTag(4), 2))),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag(8),
                        BinaryOperator::Mul,
                        Box::new(Expression::Number(NumberData::new(SrcTag(8), 9))),
                        Box::new(Expression::Number(NumberData::new(SrcTag(12), 1))),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn operator_precedence_parenthesis() {
        let program = "a = 2 * (9 + 1);";
        let ast = Expression::parse(&program).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag(0),
                Arc::new("a".into()),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag(4),
                    BinaryOperator::Mul,
                    Box::new(Expression::Number(NumberData::new(SrcTag(4), 2))),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag(9),
                        BinaryOperator::Add,
                        Box::new(Expression::Number(NumberData::new(SrcTag(9), 9))),
                        Box::new(Expression::Number(NumberData::new(SrcTag(13), 1))),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn operator_precedence_comparison() {
        let program = "a = 2 + 1 > 3 - 1;";
        let ast = Expression::parse(&program).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag(0),
                Arc::new("a".into()),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag(4),
                    BinaryOperator::GreaterThan,
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag(4),
                        BinaryOperator::Add,
                        Box::new(Expression::Number(NumberData::new(SrcTag(4), 2))),
                        Box::new(Expression::Number(NumberData::new(SrcTag(8), 1))),
                    ))),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag(12),
                        BinaryOperator::Sub,
                        Box::new(Expression::Number(NumberData::new(SrcTag(12), 3))),
                        Box::new(Expression::Number(NumberData::new(SrcTag(16), 1))),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn parse_const() {
        let program = "register test_register: u8 @ 0x8000;";
        let ast = Expression::parse(&program).expect("parse");

        let expected = vec![
            Expression::DeclareRegister(DeclareRegisterData::new(
                SrcTag(0),
                NameType::new(Arc::new("test_register".into()), Type::U8),
                0x8000,
            )),
        ];

        assert_eq!(expected, ast);
    }
}
