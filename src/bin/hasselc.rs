extern crate compiler;

#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::prelude::*;

use compiler::code::to_asm;

fn main() {
    let cli_app = clap_app!(hasselc =>
        (version: "v0.1.0")
        (author: "John DiSanti <johndisanti@gmail.com>")
        (about: "6502 Compiler for the Hassel programming language")
        (@arg RUNTIME: -r --runtime +takes_value "Tells the compiler to use a pre-configured runtime environment")
        (@arg INPUT: +required "Input source file to use")
        (@arg OUTPUT: -o --output +takes_value "Sets output file name; otherwise outputs to STDOUT"));
    let cli_matches = cli_app.get_matches();

    let input_source = {
        let mut file = match File::open(cli_matches.value_of("INPUT").unwrap()) {
            Ok(file) => file,
            Err(e) => {
                println!("Failed to open input source file: {}", e);
                return;
            }
        };
        let mut file_contents = String::new();
        if !file.read_to_string(&mut file_contents).is_ok() {
            println!("Failed to read the input source file");
            return;
        }
        file_contents
    };

    let optimized_asm = match compiler::compile(&input_source, true, true) {
        Ok(compiler_output) => {
            let symbol_table = compiler_output
                .global_symbol_table
                .as_ref()
                .unwrap()
                .read()
                .unwrap();
            to_asm(&*symbol_table, compiler_output.code_opt.as_ref().unwrap()).unwrap()
        }
        Err(error) => {
            println!("{}", error.0);
            return;
        }
    };

    match cli_matches.value_of("OUTPUT") {
        Some(output_file_name) => {
            let mut file = match File::create(output_file_name) {
                Ok(file) => file,
                Err(e) => {
                    println!("Failed to create output file: {}", e);
                    return;
                }
            };
            if !file.write_all(optimized_asm.as_bytes()).is_ok() {
                println!("Failed to write to output file");
                return;
            }
        }
        None => {
            println!("{}", optimized_asm);
        }
    }
}
