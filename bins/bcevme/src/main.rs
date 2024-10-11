use bcevme::cmd::MainCmd;
use structopt::StructOpt;

fn main() {
    if let Err(e) = MainCmd::from_args().run() {
        eprintln!("{}", e);
    }
}
