//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

use std::sync::{Arc, RwLock};
use hassel_asm::Assembler;

use ir;
use llir;
use code;
use error::{self, to_compiler_error, ErrorKind};
use parse::ast;
use symbol_table::{DefaultSymbolTable, HandleGenerator, SymbolTable};
use src_unit::SrcUnits;

#[derive(Debug)]
pub struct CompilerOutput {
    pub global_symbol_table: Arc<RwLock<SymbolTable>>,
    pub ast: Option<Vec<ast::Expression>>,
    pub ir: Option<Vec<ir::Block>>,
    pub llir: Option<Vec<llir::FrameBlock>>,
    pub code: Option<Vec<code::CodeBlock>>,
    pub asm: Option<String>,
    pub asm_map: Option<String>,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Default, Builder, Debug)]
#[builder(setter(into))]
pub struct CompilerOptions {
    #[builder(default)] pub optimize_llir: bool,

    #[builder(default)] pub optimize_code: bool,

    #[builder(default)] pub vector_reset_label: Option<String>,

    #[builder(default)] pub vector_irq_label: Option<String>,

    #[builder(default)] pub vector_nmi_label: Option<String>,
}

pub struct Compiler {
    global_symbol_table: Arc<RwLock<SymbolTable>>,
    src_units: SrcUnits,
    units: Vec<ast::Expression>,
    options: CompilerOptions,
}

impl Compiler {
    pub fn new(options: CompilerOptions) -> Compiler {
        let handle_gen = Arc::new(RwLock::new(HandleGenerator::new()));
        Compiler {
            global_symbol_table: Arc::new(RwLock::new(DefaultSymbolTable::new(handle_gen, 0))),
            src_units: SrcUnits::new(),
            units: Vec::new(),
            options: options,
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
            code: None,
            asm: None,
            asm_map: None,
            bytes: None,
        };

        match ir::generate(
            &self.global_symbol_table,
            compiler_output.ast.as_ref().unwrap(),
        ) {
            Ok(ir) => compiler_output.ir = Some(ir),
            Err(err) => return Err(to_compiler_error(&self.src_units, err, compiler_output)),
        }

        compiler_output.llir = Some(llir::generate_llir(compiler_output.ir.as_ref().unwrap())?);
        if self.options.optimize_llir {
            compiler_output.llir = Some(llir::optimize_llir(compiler_output.llir.as_ref().unwrap())?);
        }

        compiler_output.code =
            Some(code::CodeBlockGenerator::new(&self.src_units, compiler_output.llir.as_ref().unwrap()).generate()?);

        if self.options.optimize_code {
            compiler_output.code = Some(code::optimize_code(compiler_output.code.as_ref().unwrap())?);
        }

        compiler_output.asm = Some(code::to_asm(
            &*self.global_symbol_table.read().unwrap(),
            compiler_output.code.as_ref().unwrap(),
        )?);

        if self.options.vector_irq_label.is_some() || self.options.vector_nmi_label.is_some()
            || self.options.vector_reset_label.is_some()
        {
            compiler_output.asm.as_mut().unwrap().push_str(&format!(
                "\n\
                 .org\t$FFFA\n\
                 .vector\t{}\n\
                 .vector\t{}\n\
                 .vector\t{}\n\
                 ",
                self.options
                    .vector_nmi_label
                    .as_ref()
                    .map(|s| s as &str)
                    .unwrap_or("main"),
                self.options
                    .vector_reset_label
                    .as_ref()
                    .map(|s| s as &str)
                    .unwrap_or("main"),
                self.options
                    .vector_irq_label
                    .as_ref()
                    .map(|s| s as &str)
                    .unwrap_or("main"),
            ));
        }

        let mut assembler = Assembler::new();
        match assembler.parse_unit(
            "intermediate assembly",
            compiler_output.asm.as_ref().unwrap(),
        ) {
            Ok(_) => {}
            Err(err) => {
                panic!(
                    "----\n{}\n----\ngenerated invalid assembly: {}",
                    compiler_output.asm.as_ref().unwrap(),
                    err
                );
            }
        }
        let assembler_output = assembler.assemble()?;

        compiler_output.asm_map = assembler_output.source_map;
        compiler_output.bytes = assembler_output.bytes;
        Ok(compiler_output)
    }
}
