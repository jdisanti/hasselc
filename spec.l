org 0xC000;
const IOP1_ADDR = 0xDFFE;
const IOP1_SET_VALUE = 0x05;

def divide_u8(numerator: u8, divisor: u8): u8
    left_shift numerator;

    for i: u8 in 0 to 8
    	rotate_left numerator;
        if acc >= divisor
            break;
        end
        numerator = numerator - divisor;
        rotate_left numerator;
    end

    return numerator;
end

def print_chr(chr: u8): void
    poke IOP1_ADDR, IOP1_SET_VALUE
    poke IOP1_ADDR, chr
end
