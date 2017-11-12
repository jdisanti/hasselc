#![recursion_limit = "1024"]

extern crate lalrpop_util;

#[macro_use]
extern crate error_chain;

#[cfg_attr(rustfmt, rustfmt_skip)]
mod grammar;

mod ast;
mod ir;
mod ir_gen;
mod llir;
mod llir_gen;
mod llir_opt;
mod code;
mod code_gen;
mod code_opt;
mod compiler;
mod error;

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
            println!("\n\n\n\nCODE: {:#?}", compiler_output.code);
            println!("\n\n\n\nOPTIMIZED: {:#?}", compiler_output.code_opt);
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
            println!("{:#?}", error);
        }
    }
}
