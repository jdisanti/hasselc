extern crate compiler;

#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::prelude::*;
use std::process;

use compiler::code::to_asm;
use compiler::Compiler;
use compiler::error;

const RUNTIME_NONE: (&'static str, &'static str, &'static str,) = ("no_rt", "", "");
const RUNTIME_HASSELDORF: (&'static str, &'static str, &'static str) = (
    "hasseldorf_rt",
    include_str!("./runtimes/hasseldorf_prefix.hsl"),
    include_str!("./runtimes/hasseldorf_suffix.hsl"),
);
const RUNTIME_HASSELDORF_ROM: (&'static str, &'static str, &'static str) = (
    "hasseldorf_rom_rt",
    include_str!("./runtimes/hasseldorf_rom_prefix.hsl"),
    include_str!("./runtimes/hasseldorf_rom_suffix.hsl"),
);

fn die(err: error::Error) -> ! {
    println!("{}", err.0);
    process::exit(1);
}

fn handle_result<T>(result: error::Result<T>) -> T {
    match result {
        Ok(t) => t,
        Err(err) => die(err),
    }
}

fn main() {
    let cli_app = clap_app!(hasselc =>
        (version: "v0.1.0")
        (author: "John DiSanti <johndisanti@gmail.com>")
        (about: "6502 Compiler for the Hassel programming language")
        (@arg RUNTIME: -r --runtime +takes_value "Tells the compiler to use a pre-configured runtime environment")
        (@arg INPUT: +required "Input source file to use")
        (@arg OUTPUT: -o --output +takes_value "Sets output file name; otherwise outputs to STDOUT"));
    let cli_matches = cli_app.get_matches();

    let input_name = cli_matches.value_of("INPUT").unwrap();
    let input_source = {
        let mut file = match File::open(input_name) {
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

    let (rt_name, rt_prefix, rt_suffix) = if let Some(name) = cli_matches.value_of("RUNTIME") {
        'outer: loop {
            for &(rt_name, rt_prefix, rt_suffix) in &[RUNTIME_HASSELDORF, RUNTIME_HASSELDORF_ROM] {
                if rt_name == name {
                    break 'outer (rt_name, rt_prefix, rt_suffix);
                }
            }
            break RUNTIME_NONE;
        }
    } else {
        RUNTIME_NONE
    };

    let mut compiler = Compiler::new(true, true);
    handle_result(compiler.parse_unit(rt_name, rt_prefix));
    handle_result(compiler.parse_unit(input_name, &input_source));
    handle_result(compiler.parse_unit(rt_name, rt_suffix));

    let compiler_output = handle_result(compiler.compile());
    let optimized_asm = {
        let symbol_table = compiler_output.global_symbol_table.read().unwrap();
        to_asm(&*symbol_table, compiler_output.code_opt.as_ref().unwrap()).unwrap()
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
