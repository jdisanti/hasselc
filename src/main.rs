mod grammar;
mod tokens;

fn main() {
    println!("{:#?}", grammar::parse_Expression("
        a - (5 + 9) * 10 / b == 132 + 5 * 6
    "));

    println!("{:#?}", grammar::parse_Program("
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
}
