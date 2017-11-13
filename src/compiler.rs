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

#[derive(Debug, Clone)]
pub struct CompilerOutput {
    pub ast: Option<Vec<ast::Expression>>,
    pub ir: Option<Vec<ir::IR>>,
    pub llir: Option<Vec<llir::Block>>,
    pub llir_opt: Option<Vec<llir::Block>>,
    pub code: Option<Vec<code::CodeBlock>>,
    pub code_opt: Option<Vec<code::CodeBlock>>,
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

    compiler_output.ast = Some(ast::Expression::parse(program)?);

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
