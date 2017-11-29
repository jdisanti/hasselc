use std::sync::Arc;
use lalrpop_util;
use src_tag::{SrcTag, SrcTagged};
use src_unit::SrcUnit;
use type_expr::BaseType;
use error;

#[derive(Debug, Clone, Eq, PartialEq, new)]
pub struct NameType {
    pub name: Arc<String>,
    pub base_type: BaseType,
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

impl BinaryOperator {
    pub fn is_arithmetic(&self) -> bool {
        use self::BinaryOperator::*;
        match *self {
            Add | Sub | Mul | Div => true,
            _ => false,
        }
    }
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct ArrayIndexData {
    pub tag: SrcTag,
    pub array: Arc<String>,
    pub index: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct AssignmentData {
    pub tag: SrcTag,
    pub left_value: Box<Expression>,
    pub right_value: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct BinaryOpData {
    pub tag: SrcTag,
    pub op: BinaryOperator,
    pub left: Box<Expression>,
    pub right: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct CallFunctionData {
    pub tag: SrcTag,
    pub name: Arc<String>,
    pub arguments: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct ConditionalData {
    pub tag: SrcTag,
    pub condition: Box<Expression>,
    pub when_true: Vec<Expression>,
    pub when_false: Vec<Expression>,
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
    pub return_type: BaseType,
    pub body: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareRegisterData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub location: i32,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct DeclareVariableData {
    pub tag: SrcTag,
    pub name_type: NameType,
    pub value: Box<Expression>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct GoToData {
    pub tag: SrcTag,
    pub destination: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct InlineAsmData {
    pub tag: SrcTag,
    pub asm: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct NameData {
    pub tag: SrcTag,
    pub name: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct NumberData {
    pub tag: SrcTag,
    pub value: i32,
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
pub struct TextData {
    pub tag: SrcTag,
    pub value: Arc<String>,
}

#[derive(Debug, Eq, PartialEq, new)]
pub struct WhileLoopData {
    pub tag: SrcTag,
    pub condition: Box<Expression>,
    pub body: Vec<Expression>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Expression {
    ArrayIndex(ArrayIndexData),
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
    InlineAsm(InlineAsmData),
    Name(NameData),
    Number(NumberData),
    Org(OrgData),
    Return(ReturnData),
    Text(TextData),
    WhileLoop(WhileLoopData),
}

impl Expression {
    pub fn parse<'a>(src_unit: &'a SrcUnit) -> error::Result<Vec<Expression>> {
        if src_unit.source == "" {
            return Ok(Vec::new());
        }

        let mut errors: Vec<lalrpop_util::ErrorRecovery<usize, (usize, &'a str), ()>> = Vec::new();
        let ast = ::parse::grammar::parse_Program(src_unit.id, &mut errors, &src_unit.source);
        if errors.is_empty() {
            match ast {
                Ok(expression) => Ok(expression),
                Err(err) => Err(translate_errors(src_unit, [err].iter()).into()),
            }
        } else {
            Err(
                translate_errors(src_unit, errors.iter().map(|err| &err.error)).into(),
            )
        }
    }
}

impl SrcTagged for Expression {
    fn src_tag(&self) -> SrcTag {
        use self::Expression::*;
        match *self {
            ArrayIndex(ref d) => d.tag,
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
            InlineAsm(ref d) => d.tag,
            GoTo(ref d) => d.tag,
            Name(ref d) => d.tag,
            Number(ref d) => d.tag,
            Org(ref d) => d.tag,
            Return(ref d) => d.tag,
            Text(ref d) => d.tag,
            WhileLoop(ref d) => d.tag,
        }
    }
}

fn translate_errors<'a, I>(unit: &SrcUnit, errors: I) -> error::ErrorKind
where
    I: Iterator<Item = &'a lalrpop_util::ParseError<usize, (usize, &'a str), ()>>,
{
    let mut messages = Vec::new();
    for error in errors {
        match *error {
            lalrpop_util::ParseError::InvalidToken { location } => {
                let (row, col) = SrcTag::new(0, location).row_col(&unit.source);
                messages.push(format!("{}:{}:{}: invalid token", unit.name, row, col));
            }
            lalrpop_util::ParseError::UnrecognizedToken {
                ref token,
                ref expected,
            } => {
                match *token {
                    Some((start, token, _end)) => {
                        let (row, col) = SrcTag::new(0, start).row_col(&unit.source);
                        messages.push(format!(
                            "{}:{}:{}: unexpected token \"{}\". Expected one of: {:?}",
                            unit.name,
                            row,
                            col,
                            token.1,
                            expected
                        ));
                    }
                    None => {
                        messages.push(format!(
                            "{}: unexpected EOF; expected: {:?}",
                            unit.name,
                            expected
                        ));
                    }
                }
            }
            lalrpop_util::ParseError::ExtraToken { ref token } => {
                let (row, col) = SrcTag::new(0, token.0).row_col(&unit.source);
                messages.push(format!(
                    "{}:{}:{}: extra token \"{}\"",
                    unit.name,
                    row,
                    col,
                    (token.1).1
                ));
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
        let ast = Expression::parse(&SrcUnit::new(0, "".into(), program.into())).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag::new(0, 0),
                Box::new(Expression::Name(
                    NameData::new(SrcTag::new(0, 0), Arc::new("a".into())),
                )),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag::new(0, 4),
                    BinaryOperator::Add,
                    Box::new(
                        Expression::Number(NumberData::new(SrcTag::new(0, 4), 2)),
                    ),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag::new(0, 8),
                        BinaryOperator::Mul,
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 8), 9)),
                        ),
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 12), 1)),
                        ),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn operator_precedence_parenthesis() {
        let program = "a = 2 * (9 + 1);";
        let ast = Expression::parse(&SrcUnit::new(0, "".into(), program.into())).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag::new(0, 0),
                Box::new(Expression::Name(
                    NameData::new(SrcTag::new(0, 0), Arc::new("a".into())),
                )),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag::new(0, 4),
                    BinaryOperator::Mul,
                    Box::new(
                        Expression::Number(NumberData::new(SrcTag::new(0, 4), 2)),
                    ),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag::new(0, 9),
                        BinaryOperator::Add,
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 9), 9)),
                        ),
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 13), 1)),
                        ),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn operator_precedence_comparison() {
        let program = "a = 2 + 1 > 3 - 1;";
        let ast = Expression::parse(&SrcUnit::new(0, "".into(), program.into())).expect("parse");

        let expected = vec![
            Expression::Assignment(AssignmentData::new(
                SrcTag::new(0, 0),
                Box::new(Expression::Name(
                    NameData::new(SrcTag::new(0, 0), Arc::new("a".into())),
                )),
                Box::new(Expression::BinaryOp(BinaryOpData::new(
                    SrcTag::new(0, 4),
                    BinaryOperator::GreaterThan,
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag::new(0, 4),
                        BinaryOperator::Add,
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 4), 2)),
                        ),
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 8), 1)),
                        ),
                    ))),
                    Box::new(Expression::BinaryOp(BinaryOpData::new(
                        SrcTag::new(0, 12),
                        BinaryOperator::Sub,
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 12), 3)),
                        ),
                        Box::new(
                            Expression::Number(NumberData::new(SrcTag::new(0, 16), 1)),
                        ),
                    ))),
                ))),
            )),
        ];

        assert_eq!(expected, ast);
    }

    #[test]
    fn parse_const() {
        let program = "register test_register: u8 @ 0x8000;";
        let ast = Expression::parse(&SrcUnit::new(0, "".into(), program.into())).expect("parse");

        let expected = vec![
            Expression::DeclareRegister(DeclareRegisterData::new(
                SrcTag::new(0, 0),
                NameType::new(
                    Arc::new("test_register".into()),
                    BaseType::U8,
                ),
                0x8000,
            )),
        ];

        assert_eq!(expected, ast);
    }
}
