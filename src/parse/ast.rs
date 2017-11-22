use std::sync::Arc;
use lalrpop_util;
use src_tag::{SrcTag, SrcTagged};
use types::Type;
use error;

#[derive(Debug, Eq, PartialEq)]
pub enum Literal {
    Int(i32),
    Str(Arc<String>),
}

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct NameType {
    pub name: Arc<String>,
    pub type_name: Type,
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

#[derive(Debug, Eq, PartialEq, new)]
pub struct NumberData {
    pub tag: SrcTag,
    pub value: i32,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct NameData {
    pub tag: SrcTag,
    pub name: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub op: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct AssignmentData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub value: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct CallFunctionData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub arguments: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareConstData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub value: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareFunctionData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub parameters: Vec<NameType>,
    pub return_type: Type,
    pub body: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareVariableData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub value: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareRegisterData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub location: i32,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct OrgData {
    pub tag: SrcTag,
    pub address: i32,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct ReturnData {
    pub tag: SrcTag,
    pub value: Option<Box<Expression>>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Box<Expression>,
    pub when_true: Vec<Expression>,
    pub when_false: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct WhileLoopData {
    pub tag: SrcTag,
    pub condition: Box<Expression>,
    pub body: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: Arc<String>,
}

#[derive(Debug, Eq, PartialEq)]
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
    GoTo(GoToData),
    Name(NameData),
    Number(NumberData),
    Org(OrgData),
    Return(ReturnData),
    WhileLoop(WhileLoopData),
}

impl Expression {
    pub fn parse<'a>(text: &'a str) -> error::Result<Vec<Expression>> {
        let mut errors: Vec<lalrpop_util::ErrorRecovery<usize, (usize, &'a str), ()>> = Vec::new();
        let ast = ::parse::grammar::parse_Program(&mut errors, text);
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

impl SrcTagged for Expression {
    fn src_tag(&self) -> SrcTag {
        use self::Expression::*;
        match *self {
            Assignment(ref d) => d.tag,
            BinaryOp(ref d) => d.tag,
            Break => unimplemented!(),
            CallFunction(ref d) => d.tag,
            Comment => unimplemented!(),
            Conditional(ref d) => d.tag,
            DeclareConst(ref d) => d.tag,
            DeclareFunction(ref d) => d.tag,
            DeclareRegister(ref d) => d.tag,
            DeclareVariable(ref d) => d.tag,
            Error => unimplemented!(),
            GoTo(ref d) => d.tag,
            Name(ref d) => d.tag,
            Number(ref d) => d.tag,
            Org(ref d) => d.tag,
            Return(ref d) => d.tag,
            WhileLoop(ref d) => d.tag,
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
                    messages.push(String::from("unexpected EOF"));
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
    error::ErrorKind::ParseError(messages)
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
