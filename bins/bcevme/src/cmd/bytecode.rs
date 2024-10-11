use bcevm::{interpreter::opcode::eof_printer::print_eof_code, primitives::{Bytes, Eof}};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cmd {
    #[structopt(required = true)]
    bytes: String,
}

impl Cmd {
    pub fn run(&self) {
        let bytes: Bytes = match hex::decode(self.bytes.trim_start_matches("0x")) {
            Ok(b) if !b.is_empty() => b.into(),
            _ => {
                eprintln!("Invalid or empty hex string");
                return;
            }
        };

        if bytes[0] == 0xEF {
            match Eof::decode(bytes) {
                Ok(eof) => println!("{:#?}", eof),
                Err(_) => eprintln!("Invalid EOF bytecode"),
            }
        } else {
            print_eof_code(&bytes)
        }
    }
}
