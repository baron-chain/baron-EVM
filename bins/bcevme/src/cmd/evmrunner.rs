use bcevm::{db::BenchmarkDB, primitives::{Address, Bytecode, TransactTo}, Evm};
use std::{io::Error as IoError, path::PathBuf, time::Duration, borrow::Cow, fs};
use structopt::StructOpt;

#[derive(Debug, thiserror::Error)]
pub enum Errors {
    #[error("The specified path does not exist")]
    PathNotExists,
    #[error("Invalid bytecode")]
    InvalidBytecode,
    #[error("Invalid input")]
    InvalidInput,
    #[error("EVM Error")]
    EVMError,
    #[error(transparent)]
    Io(#[from] IoError),
}

#[derive(StructOpt, Debug)]
pub struct Cmd {
    #[structopt(default_value = "")]
    bytecode: String,
    #[structopt(long)]
    path: Option<PathBuf>,
    #[structopt(long)]
    bench: bool,
    #[structopt(long, default_value = "")]
    input: String,
    #[structopt(long)]
    state: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), Errors> {
        let bytecode_str: Cow<'_, str> = if let Some(path) = &self.path {
            if !path.exists() {
                return Err(Errors::PathNotExists);
            }
            fs::read_to_string(path)?.into()
        } else {
            self.bytecode.as_str().into()
        };
        let bytecode = hex::decode(bytecode_str.trim()).map_err(|_| Errors::InvalidBytecode)?;
        let input = hex::decode(self.input.trim()).map_err(|_| Errors::InvalidInput)?;

        let mut evm = Evm::builder()
            .with_db(BenchmarkDB::new_bytecode(Bytecode::new_raw(bytecode.into())))
            .modify_tx_env(|tx| {
                tx.caller = "0x0000000000000000000000000000000000000001".parse().unwrap();
                tx.transact_to = TransactTo::Call(Address::ZERO);
                tx.data = input.into();
            })
            .build();

        if self.bench {
            microbench::bench(&microbench::Options::default().time(Duration::from_secs(3)), "Run bytecode", || {
                let _ = evm.transact().unwrap();
            });
        } else {
            let out = evm.transact().map_err(|_| Errors::EVMError)?;
            println!("Result: {:#?}", out.result);
            if self.state {
                println!("State: {:#?}", out.state);
            }
        }
        Ok(())
    }
}
