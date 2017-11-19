extern crate compiler;
extern crate emulator;

use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::process;
use std::io::prelude::*;
use emulator::cpu::Cpu;
use emulator::bus::{Bus, PlaceholderBus};

pub const ROM_SIZE: usize = 0x2000;

pub struct Emulator {
    pub cpu: Box<Cpu>,
    last_pc: u16,
}

impl Emulator {
    pub fn new(mut rom: Vec<u8>) -> Emulator {
        while rom.len() < ROM_SIZE {
            rom.push(0);
        }

        // Set the start location
        rom[ROM_SIZE - 3] = 0xE0;
        rom[ROM_SIZE - 4] = 0x00;

        Emulator {
            cpu: Box::new(Cpu::new(
                rom,
                Rc::new(RefCell::new(PlaceholderBus::new("mock_peripherals".into()))),
            )),
            last_pc: 0,
        }
    }

    pub fn is_halted(&self) -> bool {
        self.last_pc == self.cpu.reg_pc()
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
    }

    pub fn step(&mut self) -> usize {
        println!("{}", self.cpu.debug_next_instruction());
        self.last_pc = self.cpu.reg_pc();
        let cycles = self.cpu.next_instruction();
        cycles
    }
}

fn to_asm(blocks: &Vec<compiler::code::CodeBlock>) -> String {
    let mut asm = String::new();
    for block in blocks {
        asm.push_str(&block.to_asm().unwrap());
    }
    asm
}

fn compile(program: &str, optimize_llir: bool, optimize_code: bool) -> compiler::CompilerOutput {
    match compiler::compile(program, optimize_llir, optimize_code) {
        Ok(compiler_output) => compiler_output,
        Err(err) => panic!("failed to compile: {:?}", err),
    }
}

#[cfg(target_os = "windows")]
fn assembler_name() -> &'static str {
    "../assembler/asm.bat"
}

#[cfg(not(target_os = "windows"))]
fn assembler_name() -> &'static str {
    "../assembler/asm"
}

fn assemble(name: &str, program: &str, optimize_llir: bool, optimize_code: bool) -> Vec<u8> {
    let compiler_output = compile(program, optimize_llir, optimize_code);

    let asm = if optimize_code {
        to_asm(compiler_output.code_opt.as_ref().unwrap())
    } else {
        to_asm(compiler_output.code.as_ref().unwrap())
    };

    println!("Program:\n{}\n", asm);

    fs::create_dir_all("test_output").expect("create_dir_all");

    let mut file = fs::File::create(format!("test_output/{}.llir", name)).unwrap();
    file.write_all(format!("{:#?}", compiler_output.llir).as_bytes()).unwrap();
    drop(file);

    let mut file = fs::File::create(format!("test_output/{}.s", name)).unwrap();
    file.write_all(asm.as_bytes()).unwrap();
    drop(file);

    let assemble_result = process::Command::new(assembler_name())
        .arg("-o")
        .arg(format!("test_output/{}.rom", name))
        .arg(format!("test_output/{}.s", name))
        .stdout(process::Stdio::piped())
        .status()
        .unwrap();
    if !assemble_result.success() {
        panic!("assembly failed");
    }

    let mut code = Vec::new();
    let mut file = fs::File::open(format!("test_output/{}.rom", name)).unwrap();
    file.read_to_end(&mut code).unwrap();

    println!("assembled to {} bytes of program code", code.len());
    code
}

fn run_test(name: &str, program_raw: &[u8], optimize_llir: bool, optimize_code: bool) -> Emulator {
    let mut program_bytes = Vec::new();
    program_bytes.extend(program_raw.iter());

    let program = String::from_utf8(program_bytes).unwrap();
    let assembled = assemble(name, &program, optimize_llir, optimize_code);

    let mut step_num = 0;
    let mut emulator = Emulator::new(assembled);
    emulator.reset();
    while !emulator.is_halted() {
        emulator.step();
        step_num += 1;

        if step_num > 1_000 {
            panic!("code under test is probably infinite looping");
        }
    }
    emulator
}

#[test]
pub fn test1_unoptimized() {
    let emulator = run_test(
        "test1_unoptimized",
        include_bytes!("./test1.hsl"),
        false,
        false,
    );
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn test1_optimized() {
    let emulator = run_test("test1_optimized", include_bytes!("./test1.hsl"), true, true);
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn simple_arg_test_unoptimized() {
    let emulator = run_test(
        "simple_arg_test_unoptimized",
        include_bytes!("./simple_arg_test.hsl"),
        false,
        false,
    );
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(30u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(40u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn simple_arg_test_optimized() {
    let emulator = run_test(
        "simple_arg_test_optimized",
        include_bytes!("./simple_arg_test.hsl"),
        true,
        true,
    );
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(30u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(40u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn multiple_calls_test_unoptimized() {
    let emulator = run_test(
        "multiple_calls_test_unoptimized",
        include_bytes!("./multiple_calls_test.hsl"),
        false,
        false,
    );
    assert_eq!(45u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn multiple_calls_test_optimized() {
    let emulator = run_test(
        "multiple_calls_test_optimized",
        include_bytes!("./multiple_calls_test.hsl"),
        true,
        true,
    );
    assert_eq!(45u8, emulator.cpu.bus.read_byte(0x0200));
}
