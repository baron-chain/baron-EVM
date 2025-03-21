pub mod bytecode;
pub mod evmrunner;
pub mod format_kzg_setup;
pub mod statetest;

use structopt::{clap::AppSettings, StructOpt};

#[derive(StructOpt, Debug)]
#[structopt(setting = AppSettings::InferSubcommands)]
#[allow(clippy::large_enum_variant)]
pub enum MainCmd {
    #[structopt(about = "Launch Ethereum state tests")]
    Statetest(statetest::Cmd),
    #[structopt(about = "Format kzg settings from a trusted setup file (.txt) into binary format (.bin)")]
    FormatKzgSetup(format_kzg_setup::Cmd),
    #[structopt(about = "Run arbitrary evm bytecode from cli or file")]
    Evm(evmrunner::Cmd),
    #[structopt(alias = "bc", about = "Print opcodes of hex Bytecodes")]
    Bytecode(bytecode::Cmd),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Statetest(#[from] statetest::Error),
    #[error(transparent)]
    KzgErrors(#[from] format_kzg_setup::KzgErrors),
    #[error(transparent)]
    EvmRunnerErrors(#[from] evmrunner::Errors),
}

impl MainCmd {
    pub fn run(&self) -> Result<(), Error> {
        match self {
            Self::Statetest(cmd) | Self::FormatKzgSetup(cmd) | Self::Evm(cmd) => cmd.run().map_err(Into::into),
            Self::Bytecode(cmd) => { cmd.run(); Ok(()) }
        }
    }
}
