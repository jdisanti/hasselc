extern crate lalrpop_util;

mod grammar;
mod ast;
mod ir;
mod ir_gen;
mod llir;
mod llir_gen;
mod code;
mod code_gen;

fn main() {
    let program = "
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

        # Call main
        #main();

        #def test(a: u8, b: u8): u8
        #    var c: u8 = a + b;
        #    return c;
        #end

        # Our main function!
        #def main(): void
        #    var my_var: u8 = 1;
        #    my_var = test(42, 10);
        #    #my_var = test(my_var, test(my_var, 12));
        #end
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
    
    let code = match code_gen::generate_code(&llir) {
        Ok(code) => code,
        Err(_) => {
            println!("Failed to generate code");
            return;
        }
    };
    println!("\n\n\n\nCODE: {:#?}", code);
}
