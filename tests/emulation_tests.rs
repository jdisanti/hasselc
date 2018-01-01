//
// Copyright 2017 hasselc Developers
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
//

extern crate hasselc;
extern crate hassel_emu;

use std::rc::Rc;
use std::cell::RefCell;
use std::fs;
use std::io::prelude::*;
use hassel_emu::cpu::Cpu;
use hassel_emu::bus::{Bus, PlaceholderBus};
use hasselc::error;

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
                Rc::new(
                    RefCell::new(PlaceholderBus::new("mock_peripherals".into())),
                ),
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
        self.cpu.next_instruction()
    }
}

fn compile(
    name: &str,
    program: &str,
    optimize_llir: bool,
    optimize_code: bool,
) -> error::Result<hasselc::CompilerOutput> {
    let compiler_options = hasselc::CompilerOptionsBuilder::default()
        .optimize_llir(optimize_llir)
        .optimize_code(optimize_code)
        .build()
        .unwrap();
    let mut compiler = hasselc::Compiler::new(compiler_options);
    compiler.parse_unit(name, program)?;
    compiler.compile()
}

fn assemble(name: &str, program: &str, optimize_llir: bool, optimize_code: bool) -> Vec<u8> {
    let compiler_output = match compile(name, program, optimize_llir, optimize_code) {
        Ok(output) => output,
        Err(err) => panic!(format!("{}", err.0)),
    };

    println!("Program:\n{}\n", compiler_output.asm.as_ref().unwrap());

    fs::create_dir_all("test_output").expect("create_dir_all");

    let mut file = fs::File::create(format!("test_output/{}.llir", name)).unwrap();
    file.write_all(format!("{:#?}", compiler_output.llir).as_bytes()).unwrap();
    drop(file);

    let mut file = fs::File::create(format!("test_output/{}.s", name)).unwrap();
    file.write_all(compiler_output.asm.unwrap().as_bytes()).unwrap();
    drop(file);

    let mut file = fs::File::create(format!("test_output/{}.rom", name)).unwrap();
    file.write_all(compiler_output.bytes.as_ref().unwrap()).unwrap();
    drop(file);

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

macro_rules! emulate {
    (optimized : $test_name:ident) => {
        run_test(
            concat!(stringify!($test_name), "_optimized"),
            include_bytes!(concat!("./", stringify!($test_name), ".hsl")),
            true,
            true,
        );
    };
    (unoptimized : $test_name:ident) => {
        run_test(
            concat!(stringify!($test_name), "_unoptimized"),
            include_bytes!(concat!("./", stringify!($test_name), ".hsl")),
            false,
            false,
        );
    };
}

#[test]
pub fn test1_unoptimized() {
    let emulator = emulate!(unoptimized: test1);
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn test1_optimized() {
    let emulator = emulate!(optimized: test1);
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn simple_arg_test_unoptimized() {
    let emulator = emulate!(unoptimized: simple_arg_test);
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(30u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(40u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn simple_arg_test_optimized() {
    let emulator = emulate!(optimized: simple_arg_test);
    assert_eq!(20u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(30u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(40u8, emulator.cpu.bus.read_byte(0x0001));
}

#[test]
pub fn two_calls_test_unoptimized() {
    let emulator = emulate!(unoptimized: two_calls_test);
    assert_eq!(21u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn two_calls_test_optimized() {
    let emulator = emulate!(optimized: two_calls_test);
    assert_eq!(21u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn three_calls_test_unoptimized() {
    let emulator = emulate!(unoptimized: three_calls_test);
    assert_eq!(45u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn three_calls_test_optimized() {
    let emulator = emulate!(optimized: three_calls_test);
    assert_eq!(45u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn simple_branch_test_unoptimized() {
    let emulator = emulate!(unoptimized: simple_branch_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(6u8, emulator.cpu.bus.read_byte(0x0201));
}

#[test]
pub fn simple_branch_test_optimized() {
    let emulator = emulate!(optimized: simple_branch_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(6u8, emulator.cpu.bus.read_byte(0x0201));
}

#[test]
pub fn recursion_test_unoptimized() {
    let emulator = emulate!(unoptimized: recursion_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn recursion_test_optimized() {
    let emulator = emulate!(optimized: recursion_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn while_loop_test_unoptimized() {
    let emulator = emulate!(unoptimized: while_loop_test);
    assert_eq!(10u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn while_loop_test_optimized() {
    let emulator = emulate!(optimized: while_loop_test);
    assert_eq!(10u8, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn comparison_test_unoptimized() {
    let emulator = emulate!(unoptimized: comparison_test);
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0200), "eq_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0201), "eq_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0202), "neq_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0203), "neq_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0204), "lt_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0205), "lt_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0206), "lte_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0207), "lte_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0208), "gt_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0209), "gt_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x020A), "gte_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x020B), "gte_false");
}

#[test]
pub fn comparison_test_optimized() {
    let emulator = emulate!(optimized: comparison_test);
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0200), "eq_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0201), "eq_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0202), "neq_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0203), "neq_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0204), "lt_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0205), "lt_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0206), "lte_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0207), "lte_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x0208), "gt_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x0209), "gt_false");
    assert_eq!(1u8, emulator.cpu.bus.read_byte(0x020A), "gte_true");
    assert_eq!(0u8, emulator.cpu.bus.read_byte(0x020B), "gte_false");
}

#[test]
pub fn conditions_test_unoptimized() {
    let emulator = emulate!(unoptimized: conditions_test);
    assert_eq!(42u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(49u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(22u8, emulator.cpu.bus.read_byte(0x0202));
}

#[test]
pub fn conditions_test_optimized() {
    let emulator = emulate!(optimized: conditions_test);
    assert_eq!(42u8, emulator.cpu.bus.read_byte(0x0200), "output1");
    assert_eq!(49u8, emulator.cpu.bus.read_byte(0x0201), "output2");
    assert_eq!(22u8, emulator.cpu.bus.read_byte(0x0202), "output3");
}

#[test]
pub fn no_op_test_unoptimized() {
    drop(emulate!(unoptimized: no_op_test));
}

#[test]
pub fn no_op_test_optimized() {
    drop(emulate!(optimized: no_op_test));
}

#[test]
pub fn constants_test_unoptimized() {
    let emulator = emulate!(unoptimized: constants_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(15u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(3u8, emulator.cpu.bus.read_byte(0x0202));
}

#[test]
pub fn constants_test_optimized() {
    let emulator = emulate!(optimized: constants_test);
    assert_eq!(5u8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(15u8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(3u8, emulator.cpu.bus.read_byte(0x0202));
}

#[test]
pub fn word_test_unoptimized() {
    let emulator = emulate!(unoptimized: word_test);
    assert_eq!(0xBBu8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(0xAAu8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(0x00u8, emulator.cpu.bus.read_byte(0x0202));
    assert_eq!(0x01u8, emulator.cpu.bus.read_byte(0x0203));
    assert_eq!(0x04u8, emulator.cpu.bus.read_byte(0x0204));
    assert_eq!(0x00u8, emulator.cpu.bus.read_byte(0x0205));
    assert_eq!(0x58u8, emulator.cpu.bus.read_byte(0x0206));
    assert_eq!(0x02u8, emulator.cpu.bus.read_byte(0x0207));
}

#[test]
pub fn word_test_optimized() {
    let emulator = emulate!(optimized: word_test);
    assert_eq!(0xBBu8, emulator.cpu.bus.read_byte(0x0200));
    assert_eq!(0xAAu8, emulator.cpu.bus.read_byte(0x0201));
    assert_eq!(0x00u8, emulator.cpu.bus.read_byte(0x0202));
    assert_eq!(0x01u8, emulator.cpu.bus.read_byte(0x0203));
    assert_eq!(0x04u8, emulator.cpu.bus.read_byte(0x0204));
    assert_eq!(0x00u8, emulator.cpu.bus.read_byte(0x0205));
    assert_eq!(0x58u8, emulator.cpu.bus.read_byte(0x0206));
    assert_eq!(0x02u8, emulator.cpu.bus.read_byte(0x0207));
}

#[test]
pub fn array_test_unoptimized() {
    let emulator = emulate!(unoptimized: array_test);
    for i in 0..10u16 {
        assert_eq!((i + 1) as u8, emulator.cpu.bus.read_byte(0x0200 + i));
    }
    assert_eq!(6, emulator.cpu.bus.read_byte(0x0250));
}

#[test]
pub fn array_test_optimized() {
    let emulator = emulate!(optimized: array_test);
    for i in 0..10u16 {
        assert_eq!((i + 1) as u8, emulator.cpu.bus.read_byte(0x0200 + i));
    }
    assert_eq!(6, emulator.cpu.bus.read_byte(0x0250));
}

#[test]
pub fn string_test_unoptimized() {
    let emulator = emulate!(unoptimized: string_test);
    assert_eq!(12, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn string_test_optimized() {
    let emulator = emulate!(optimized: string_test);
    assert_eq!(12, emulator.cpu.bus.read_byte(0x0200));
}

#[test]
pub fn memcpy_test_unoptimized() {
    let emulator = emulate!(unoptimized: memcpy_test);
    assert_eq!('h', emulator.cpu.bus.read_byte(0x0200) as char);
    assert_eq!('e', emulator.cpu.bus.read_byte(0x0201) as char);
    assert_eq!('l', emulator.cpu.bus.read_byte(0x0202) as char);
    assert_eq!('l', emulator.cpu.bus.read_byte(0x0203) as char);
    assert_eq!('o', emulator.cpu.bus.read_byte(0x0204) as char);
    assert_eq!(0, emulator.cpu.bus.read_byte(0x0205));
}

#[test]
pub fn memcpy_test_optimized() {
    let emulator = emulate!(optimized: memcpy_test);
    assert_eq!('h', emulator.cpu.bus.read_byte(0x0200) as char);
    assert_eq!('e', emulator.cpu.bus.read_byte(0x0201) as char);
    assert_eq!('l', emulator.cpu.bus.read_byte(0x0202) as char);
    assert_eq!('l', emulator.cpu.bus.read_byte(0x0203) as char);
    assert_eq!('o', emulator.cpu.bus.read_byte(0x0204) as char);
    assert_eq!(0, emulator.cpu.bus.read_byte(0x0205));
}
