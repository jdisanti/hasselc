use std::sync::{Arc, RwLock};
use ir;
use llir;
use code;
use error::{self, to_compiler_error};
use parse::ast;
use symbol_table::{DefaultSymbolTable, HandleGenerator, SymbolTable};
use src_unit::SrcUnits;

#[derive(Debug)]
pub struct CompilerOutput {
    pub global_symbol_table: Arc<RwLock<SymbolTable>>,
    pub ast: Option<Vec<ast::Expression>>,
    pub ir: Option<Vec<ir::Block>>,
    pub llir: Option<Vec<llir::FrameBlock>>,
    pub llir_opt: Option<Vec<llir::FrameBlock>>,
    pub code: Option<Vec<code::CodeBlock>>,
    pub code_opt: Option<Vec<code::CodeBlock>>,
}

pub struct Compiler {
    global_symbol_table: Arc<RwLock<SymbolTable>>,
    src_units: SrcUnits,
    units: Vec<ast::Expression>,
    optimize_llir: bool,
    optimize_code: bool,
}

impl Compiler {
    pub fn new(optimize_llir: bool, optimize_code: bool) -> Compiler {
        let handle_gen = Arc::new(RwLock::new(HandleGenerator::new()));
        Compiler {
            global_symbol_table: Arc::new(RwLock::new(DefaultSymbolTable::new(handle_gen, 0))),
            src_units: SrcUnits::new(),
            units: Vec::new(),
            optimize_llir: optimize_llir,
            optimize_code: optimize_code,
        }
    }

    pub fn parse_unit(&mut self, unit_name: &str, unit: &str) -> error::Result<()> {
        let unit_id = self.src_units.push_unit(unit_name.into(), unit.into());
        let expressions = ast::Expression::parse(self.src_units.unit(unit_id))?;
        self.units.extend(expressions.into_iter());
        Ok(())
    }

    pub fn compile(self) -> error::Result<CompilerOutput> {
        let mut compiler_output = CompilerOutput {
            global_symbol_table: Arc::clone(&self.global_symbol_table),
            ast: Some(self.units),
            ir: None,
            llir: None,
            llir_opt: None,
            code: None,
            code_opt: None,
        };

        match ir::generate(
            &self.global_symbol_table,
            compiler_output.ast.as_ref().unwrap(),
        ) {
            Ok(ir) => compiler_output.ir = Some(ir),
            Err(err) => return Err(to_compiler_error(&self.src_units, err, compiler_output).into()),
        }

        compiler_output.llir = Some(llir::generate_llir(compiler_output.ir.as_ref().unwrap())?);
        if self.optimize_llir {
            compiler_output.llir_opt = Some(llir::optimize_llir(compiler_output.llir.as_ref().unwrap())?);
        }

        compiler_output.code = Some(code::CodeBlockGenerator::new(
            &self.src_units,
            compiler_output
                .llir_opt
                .as_ref()
                .or_else(|| compiler_output.llir.as_ref())
                .unwrap(),
        ).generate()?);

        if self.optimize_code {
            compiler_output.code_opt = Some(code::optimize_code(compiler_output.code.as_ref().unwrap())?);
        }

        Ok(compiler_output)
    }
}
