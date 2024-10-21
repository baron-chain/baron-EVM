#[cfg(feature = "std")]
pub fn print_eof_code(code: &[u8]) {
    use super::*;
    use crate::instructions::utility::read_i16;
    use bcevm_primitives::hex;

    let mut i = 0;
    while i < code.len() {
        let op = code[i];
        if let Some(opcode) = OPCODE_INFO_JUMPTABLE.get(op as usize) {
            if i + opcode.immediate_size() as usize >= code.len() {
                println!("Malformed code: immediate out of bounds");
                break;
            }

            print!("{}", opcode.name());
            if opcode.immediate_size() != 0 {
                print!(
                    " : 0x{}",
                    hex::encode(&code[i + 1..i + 1 + opcode.immediate_size() as usize])
                );
            }

            let mut additional_immediates = 0;
            if op == RJUMPV {
                let max_index = code[i + 1] as usize;
                let len = max_index + 1;
                additional_immediates = len * 2;

                if i + 1 + additional_immediates >= code.len() {
                    println!("Malformed code: immediate out of bounds");
                    break;
                }

                for vtablei in 0..len {
                    let offset = read_i16(&code[i + 2 + 2 * vtablei..]) as isize;
                    println!("RJUMPV[{vtablei}]: 0x{offset:04X}({offset})");
                }
            }

            i += 1 + opcode.immediate_size() as usize + additional_immediates;
        } else {
            println!("Unknown opcode: 0x{:02X}", op);
            i += 1;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bcevm_primitives::hex;

    #[test]
    fn sanity_test() {
        print_eof_code(&hex!("6001e200ffff00"));
    }
}
