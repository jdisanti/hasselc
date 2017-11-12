use ast;
use ir;
use ir_gen;
use llir;
use llir_gen;
use llir_opt;
use code;
use code_gen;
use code_opt;
use error;
use lalrpop_util;

#[derive(Debug, Clone)]
pub struct CompilerOutput {
    pub ast: Option<Vec<ast::Expression>>,
    pub ir: Option<Vec<ir::IR>>,
    pub llir: Option<Vec<llir::Block>>,
    pub llir_opt: Option<Vec<llir::Block>>,
    pub code: Option<Vec<code::CodeBlock>>,
    pub code_opt: Option<Vec<code::CodeBlock>>,
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

pub fn compile(program: &str, optimize_llir: bool, optimize_code: bool) -> error::Result<CompilerOutput> {
    let mut compiler_output = CompilerOutput {
        ast: None,
        ir: None,
        llir: None,
        llir_opt: None,
        code: None,
        code_opt: None,
    };

    compiler_output.ast = match ast::Expression::parse(program) {
        Ok(ast) => Some(ast),
        Err(errors) => {
            let mut messages = Vec::new();
            for error in errors {
                match error.error {
                    lalrpop_util::ParseError::InvalidToken { location } => {
                        let (row, col) = offset_to_row_col(program, location);
                        messages.push(format!("{}:{}: invalid token", row, col));
                    }
                    lalrpop_util::ParseError::UnrecognizedToken { token, expected } => match token {
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
                    lalrpop_util::ParseError::ExtraToken { token } => {
                        let (row, col) = offset_to_row_col(program, token.0);
                        messages.push(format!("{}:{}: extra token \"{}\"", row, col, (token.1).1));
                    }
                    lalrpop_util::ParseError::User { error } => {
                        messages.push(format!("{:?}", error));
                    }
                }
            }
            return Err(error::ErrorKind::ParseError(messages).into());
        }
    };

    compiler_output.ir = Some(ir_gen::generate_ir(compiler_output.ast.as_ref().unwrap())
        .map_err(|err| {
            error::ErrorKind::CompilerError(Box::new(err), compiler_output.clone())
        })?);
    compiler_output.llir = Some(llir_gen::generate_llir(
        compiler_output.ir.as_ref().unwrap(),
    )?);

    if optimize_llir {
        compiler_output.llir_opt = Some(llir_opt::optimize_llir(
            compiler_output.llir.as_ref().unwrap(),
        )?);
    }

    compiler_output.code = Some(code_gen::generate_code(
        compiler_output
            .llir_opt
            .as_ref()
            .or(compiler_output.llir.as_ref())
            .unwrap(),
    )?);

    if optimize_code {
        compiler_output.code_opt = Some(code_opt::optimize_code(
            compiler_output.code.as_ref().unwrap(),
        )?);
    }

    Ok(compiler_output)
}
