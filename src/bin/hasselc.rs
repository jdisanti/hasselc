//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

extern crate hasselc;

extern crate clap;

use std::fs::File;
use std::io::prelude::*;
use std::process;

use hasselc::{Compiler, CompilerOptions, CompilerOptionsBuilder};
use hasselc::error;

fn die(err: &error::Error) -> ! {
    println!("{}", err.0);
    process::exit(1);
}

fn handle_result<T>(result: error::Result<T>) -> T {
    match result {
        Ok(t) => t,
        Err(err) => die(&err),
    }
}

struct Options {
    compiler_options: CompilerOptions,
    input_name: String,
    output_name: Option<String>,
}

fn get_options() -> Options {
    let cli_app = clap::App::new("hasselc")
        .version("v0.1.0")
        .author("John DiSanti <johndisanti@gmail.com>")
        .about("6502 Compiler for the Hassel programming language")
        .arg(
            clap::Arg::with_name("RUNTIME")
                .short("r")
                .long("runtime")
                .value_name("RUNTIME")
                .help("Tells the compiler to use a pre-configured runtime environment")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("OUTPUT")
                .short("o")
                .long("output")
                .value_name("OUTPUT")
                .help("Sets output file name; otherwise outputs to STDOUT")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("OPTIMIZE")
                .short("O")
                .value_name("OPTIMIZE")
                .help("Sets optimization level")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("VECTOR_RESET")
                .long("vector-reset")
                .value_name("OPTIMIZE")
                .help("Generates a reset vector pointing to the given label")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("VECTOR_IRQ")
                .long("vector-irq")
                .value_name("VECTOR_IRQ")
                .help("Generates an IRQ vector pointing to the given label")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("VECTOR_NMI")
                .long("vector-nmi")
                .value_name("VECTOR_NMI")
                .help("Generates a NMI vector pointing to the given label")
                .takes_value(true),
        )
        .arg(
            clap::Arg::with_name("INPUT")
                .help("Input source file to use")
                .required(true),
        );
    let cli_matches = cli_app.get_matches();

    let mut compiler_options = CompilerOptionsBuilder::default();
    match cli_matches.value_of("OPTIMIZE") {
        Some("1") => {
            compiler_options.optimize_llir(true);
        }
        Some("2") => {
            compiler_options.optimize_llir(true);
            compiler_options.optimize_code(true);
        }
        _ => {}
    }

    compiler_options.vector_reset_label(cli_matches.value_of("VECTOR_RESET").map(String::from));
    compiler_options.vector_irq_label(cli_matches.value_of("VECTOR_IRQ").map(String::from));
    compiler_options.vector_nmi_label(cli_matches.value_of("VECTOR_NMI").map(String::from));

    Options {
        compiler_options: compiler_options.build().unwrap(),
        input_name: cli_matches.value_of("INPUT").unwrap().into(),
        output_name: cli_matches.value_of("OUTPUT").map(String::from),
    }
}

fn main() {
    let options = get_options();

    let input_source = {
        let mut file = match File::open(&options.input_name) {
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

    let mut compiler = Compiler::new(options.compiler_options);
    handle_result(compiler.parse_unit(&options.input_name, &input_source));

    let compiler_output = handle_result(compiler.compile());

    let output_file_name = options.output_name.unwrap_or_else(|| "out.rom".into());
    let asm_file_name = format!("{}.s", output_file_name);
    let asm_map_file_name = format!("{}.s.map", output_file_name);

    save_bytes(&output_file_name, &compiler_output.bytes.unwrap());
    save_bytes(
        &asm_file_name,
        &compiler_output.asm.as_ref().unwrap().as_bytes(),
    );
    save_bytes(
        &asm_map_file_name,
        &compiler_output.asm_map.as_ref().unwrap().as_bytes(),
    );
}

fn save_bytes(file_name: &str, bytes: &[u8]) {
    let mut file = match File::create(file_name) {
        Ok(file) => file,
        Err(e) => {
            println!("Failed to create output file: {}", e);
            process::exit(1);
        }
    };
    if !file.write_all(bytes).is_ok() {
        println!("Failed to write to output file");
        process::exit(1);
    }
}
