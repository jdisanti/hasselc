mod grammar;
mod tokens;

fn main() {
    println!("{:?}", grammar::parse_Text("\"test\""));
    println!("{:?}", grammar::parse_Text("\"test\\\"strings\\\"\""));
    /*println!("{:?}", grammar::parse_expression("break;"));
    println!("{:?}", grammar::parse_expression("org 0xC000;"));
    println!("{:?}", grammar::parse_expression("const TEST = 0xFF;"));*/

    println!("{:#?}", grammar::parse_program("
    org 0xC000;
    const IOP1_ADDR = 0xDFFE;
    const IOP1_SET_VALUE = 0x05;

    def test(numerator: u8, divisor: u8): u8
        left_shift numerator;
        for i: u8 in 0 to 8
            rotate_right numerator;
        end
        return 5;
    end
    "));
}
