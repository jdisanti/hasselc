#![recursion_limit = "1024"]

extern crate lalrpop_util;
extern crate num_traits;

#[macro_use]
extern crate derive_new;

#[macro_use]
extern crate error_chain;

mod symbol_table;
mod types;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod parse;
pub mod src_tag;

use code::to_asm;

fn main() {
    let program = "
        # Declare stack frame locations
        register data_stack_pointer: u8 @ 0x0000;

        register output1: u8 @ 0x0200;
        register output2: u8 @ 0x0201;

        # Initialize the stack
        org 0xE000;
        data_stack_pointer = 3;
        main();

        def halt(): void
            goto halt;
        end

        def main(): void
            var a: u8 = 0;
            while a == 0 do
                a = a + 1;
            end

            goto halt;
        end
    ";

    match compiler::compile(program, true, true) {
        Ok(compiler_output) => {
            println!("AST: {:#?}", compiler_output.ast);
            println!("\n\n\n\nIR: {:#?}", compiler_output.ir);
            println!("\n\n\n\nLLIR: {:#?}", compiler_output.llir);
            println!("\n\n\n\nOPTIMIZED LLIR: {:#?}", compiler_output.llir_opt);

            let symbol_table = compiler_output
                .global_symbol_table
                .as_ref()
                .unwrap()
                .read()
                .unwrap();
            println!(
                "\n\n\n\nCODE:\n\n{}",
                to_asm(&*symbol_table, compiler_output.code.as_ref().unwrap()).unwrap()
            );
            println!(
                "\n\n\n\nOPTIMIZED:\n\n{}",
                to_asm(&*symbol_table, compiler_output.code_opt.as_ref().unwrap()).unwrap()
            );
            let unoptimized_count = compiler_output
                .code
                .unwrap()
                .iter()
                .map(|b| b.body.len())
                .fold(0, |acc, n| acc + n) as isize;
            let optimized_count = compiler_output
                .code_opt
                .unwrap()
                .iter()
                .map(|b| b.body.len())
                .fold(0, |acc, n| acc + n) as isize;
            println!(
                "Removed {} instructions",
                unoptimized_count - optimized_count
            );
        }
        Err(error) => {
            println!("{}", error.0);
        }
    }
}
