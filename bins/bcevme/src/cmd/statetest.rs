pub mod merkle_trie;
pub mod models;
mod runner;
pub mod utils;

pub use runner::TestError as Error;
use runner::{find_all_json_tests, run, TestError};
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
pub struct Cmd {
    #[structopt(required = true)]
    path: Vec<PathBuf>,
    #[structopt(short = "s", long)]
    single_thread: bool,
    #[structopt(long)]
    json: bool,
    #[structopt(short = "o", long)]
    json_outcome: bool,
    #[structopt(long, alias = "no-fail-fast")]
    keep_going: bool,
}

impl Cmd {
    pub fn run(&self) -> Result<(), TestError> {
        for path in &self.path {
            println!("\nRunning tests in {}...", path.display());
            run(
                find_all_json_tests(path),
                self.single_thread,
                self.json,
                self.json_outcome,
                self.keep_going,
            )?
        }
        Ok(())
    }
}
