extern crate lalrpop_util;

mod grammar;
mod ast;

fn main() {
    let mut errors = Vec::new();
    println!("{:#?}", grammar::parse_Program(&mut errors, "
    org 0xC000;
    main();

    def test(a: u8, b: u8): u8
        var c: u8 = a + b;
        return c;
    end

    def main(): void
        var my_var: u8 = 5;
        my_var = test(my_var, test(my_var, 12));
    end
    "));
    println!("{:#?}", errors);
}
