use bcevm::{
    db::BenchmarkDB,
    primitives::{Bytecode, TransactTo, U256},
    Evm,
};
use std::time::Duration;

fn main() {
    let mut evm = Evm::builder()
        .with_db(BenchmarkDB::new_bytecode(Bytecode::new()))
        .modify_tx_env(|tx| {
            tx.caller = "0x0000000000000000000000000000000000000001".parse().unwrap();
            tx.value = U256::from(10);
            tx.transact_to = TransactTo::Call("0x0000000000000000000000000000000000000000".parse().unwrap());
        })
        .build();

    let bench_options = microbench::Options::default().time(Duration::from_secs(3));
    microbench::bench(&bench_options, "Simple value transfer", || {
        let _ = evm.transact().unwrap();
    });
}
