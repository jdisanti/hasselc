extern crate lalrpop_util;

mod grammar;
mod ast;
mod ir;
mod ir_gen;

fn main() {
    let program = "
        org 0xC000;
        main(); # call main

        def test(a: u8, b: u8): u8
            var c: u8 = a + b;
            return c;
        end

        # Our main function!
        def main(): void
            var my_var: u8 = 5;
            my_var = test(my_var, test(my_var, 12));
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
    println!("IR: {:#?}", ir);
}
