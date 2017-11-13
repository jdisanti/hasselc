#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate error_chain;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

mod code_gen;
mod code_opt;
mod ir_gen;
mod llir_gen;
mod llir_opt;
pub mod ast;
pub mod code;
pub mod compiler;
pub mod error;
pub mod ir;
pub mod llir;
pub mod src_tag;

fn to_asm(blocks: &Vec<code::CodeBlock>) -> String {
    let mut asm = String::new();
    for block in blocks {
        asm.push_str(&block.to_asm().unwrap());
    }
    asm
}

fn main() {
    /*let program = "
        # Declare stack frame locations
        register data_stack_pointer: u8 @ 0x0000;

        # Initialize the stack
        org 0xC000;
        data_stack_pointer = 3;

        test(3);

        def test(a: u8): u8
            var foo: u8 = 10 + a;
            return 4 + a + foo;
        end
    ";*/

    let program = "
        # Declare stack frame locations
        register data_stack_pointer: u8 @ 0x0000;

        # Initialize the stack
        org 0x0600;
        data_stack_pointer = 3;

        main();

        def test(a: u8, b: u8): u8
            var c: u8 = a + b;
            return c;
        end

        def main(): u8
            var my_var: u8 = 1;
            my_var = test(my_var, test(my_var, 18));
            return my_var;
        end
    ";

    match compiler::compile(program, true, true) {
        Ok(compiler_output) => {
            println!("AST: {:#?}", compiler_output.ast);
            println!("\n\n\n\nIR: {:#?}", compiler_output.ir);
            println!("\n\n\n\nLLIR: {:#?}", compiler_output.llir);
            println!("\n\n\n\nOPTIMIZED LLIR: {:#?}", compiler_output.llir_opt);
            println!(
                "\n\n\n\nCODE:\n\n{}",
                to_asm(compiler_output.code.as_ref().unwrap())
            );
            println!(
                "\n\n\n\nOPTIMIZED:\n\n{}",
                to_asm(compiler_output.code_opt.as_ref().unwrap())
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
