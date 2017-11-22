use ir;
use llir;
use code;
use error;
use parse::ast;

#[derive(Debug)]
pub struct CompilerOutput {
    pub ast: Option<Vec<ast::Expression>>,
    pub ir: Option<Vec<ir::Block>>,
    pub llir: Option<Vec<llir::FrameBlock>>,
    pub llir_opt: Option<Vec<llir::FrameBlock>>,
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

    match ir::generate(compiler_output.ast.as_ref().unwrap()) {
        Ok(ir) => compiler_output.ir = Some(ir),
        Err(err) => return Err(error::to_compiler_error(program, err, compiler_output)),
    }

    compiler_output.llir = Some(llir::generate_llir(compiler_output.ir.as_ref().unwrap())?);

    if optimize_llir {
        compiler_output.llir_opt = Some(llir::optimize_llir(compiler_output.llir.as_ref().unwrap())?);
    }

    compiler_output.code = Some(code::CodeBlockGenerator::new(
        program,
        compiler_output
            .llir_opt
            .as_ref()
            .or_else(|| compiler_output.llir.as_ref())
            .unwrap(),
    ).generate()?);

    if optimize_code {
        compiler_output.code_opt = Some(code::optimize_code(compiler_output.code.as_ref().unwrap())?);
    }

    Ok(compiler_output)
}
