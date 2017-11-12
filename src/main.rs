extern crate lalrpop_util;

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

    let ast = match ast::Expression::parse(program) {
        Ok(ast) => ast,
        Err(errors) => {
            println!("Syntax error(s): {:#?}", errors);
            return;
        }
    };
    println!("AST: {:#?}", ast);

    let ir = match ir_gen::generate_ir(&ast) {
        Ok(ir) => ir,
        Err(_) => {
            println!("Failed to generate IR");
            return;
        }
    };
    println!("\n\n\n\nIR: {:#?}", ir);

    let llir = match llir_gen::generate_llir(&ir) {
        Ok(llir) => llir,
        Err(_) => {
            println!("Failed to generate LLIR");
            return;
        }
    };
    println!("\n\n\n\nLLIR: {:#?}", llir);

    let optimized_llir = match llir_opt::optimize_llir(&llir) {
        Ok(llir) => llir,
        Err(_) => {
            println!("Failed to optimize LLIR");
            return;
        }
    };
    println!("\n\n\n\nOPTIMIZED LLIR: {:#?}", optimized_llir);

    let code = match code_gen::generate_code(&optimized_llir) {
        Ok(code) => code,
        Err(_) => {
            println!("Failed to generate code");
            return;
        }
    };
    println!("\n\n\n\nCODE: {:#?}", code);

    let optimized = match code_opt::optimize_code(&code) {
        Ok(opt) => opt,
        Err(_) => {
            println!("Failed to optimize code");
            return;
        }
    };
    println!("\n\n\n\nOPTIMIZED: {:#?}", optimized);

    let unoptimized_count = code.iter().map(|b| b.body.len()).fold(0, |acc, n| acc + n) as isize;
    let optimized_count = optimized
        .iter()
        .map(|b| b.body.len())
        .fold(0, |acc, n| acc + n) as isize;
    println!(
        "Removed {} instructions",
        unoptimized_count - optimized_count
    );
}
