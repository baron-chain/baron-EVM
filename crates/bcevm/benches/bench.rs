use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};
use bcevm::{
    db::BenchmarkDB,
    interpreter::{analysis::to_analysed, Contract, DummyHost, Interpreter},
    primitives::{address, bytes, hex, BerlinSpec, Bytecode, Bytes, TransactTo, U256},
    Evm,
};
use bcevm_interpreter::{opcode::make_instruction_table, SharedMemory, EMPTY_SHARED_MEMORY};
use std::time::Duration;

fn analysis(c: &mut Criterion) {
    let evm = create_evm(address!("0000000000000000000000000000000000000002"), bytes!("8035F0CE"));
    let contract_data: Bytes = hex::decode(ANALYSIS).unwrap().into();

    let mut g = create_benchmark_group(c, "analysis");

    bench_bytecode(&mut g, &evm, contract_data.clone(), Bytecode::new_raw);
    bench_bytecode(&mut g, &evm, contract_data.clone(), Bytecode::new_raw);
    bench_bytecode(&mut g, &evm, contract_data, |data| to_analysed(Bytecode::new_raw(data)));

    g.finish();
}

fn snailtracer(c: &mut Criterion) {
    let mut evm = create_evm(address!("1000000000000000000000000000000000000000"), bytes!("30627b7c"))
        .with_db(BenchmarkDB::new_bytecode(bytecode(SNAILTRACER)))
        .build();

    let mut g = create_benchmark_group(c, "snailtracer");
    bench_transact(&mut g, &mut evm);
    bench_eval(&mut g, &mut evm);
    g.finish();
}

fn transfer(c: &mut Criterion) {
    let mut evm = Evm::builder()
        .with_db(BenchmarkDB::new_bytecode(Bytecode::new()))
        .modify_tx_env(|tx| {
            tx.caller = address!("0000000000000000000000000000000000000001");
            tx.transact_to = TransactTo::Call(address!("0000000000000000000000000000000000000000"));
            tx.value = U256::from(10);
        })
        .build();

    let mut g = create_benchmark_group(c, "transfer");
    bench_transact(&mut g, &mut evm);
    g.finish();
}

fn create_evm(caller: [u8; 20], data: Vec<u8>) -> Evm<'static, (), BenchmarkDB> {
    Evm::builder()
        .modify_tx_env(|tx| {
            tx.caller = caller;
            tx.transact_to = TransactTo::Call(address!("0000000000000000000000000000000000000000"));
            tx.data = data;
        })
        .build()
}

fn create_benchmark_group(c: &mut Criterion, name: &str) -> BenchmarkGroup<criterion::measurement::WallTime> {
    c.benchmark_group(name)
        .noise_threshold(0.03)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10))
        .sample_size(10)
}

fn bench_bytecode<F>(g: &mut BenchmarkGroup<criterion::measurement::WallTime>, evm: &Evm<'static, (), BenchmarkDB>, data: Bytes, bytecode_fn: F)
where
    F: Fn(Bytes) -> Bytecode,
{
    let mut evm = evm.modify().reset_handler_with_db(BenchmarkDB::new_bytecode(bytecode_fn(data))).build();
    bench_transact(g, &mut evm);
}

fn bench_transact<EXT>(g: &mut BenchmarkGroup<criterion::measurement::WallTime>, evm: &mut Evm<'_, EXT, BenchmarkDB>) {
    let state = match evm.context.evm.db.0 {
        Bytecode::LegacyRaw(_) => "raw",
        Bytecode::LegacyAnalyzed(_) => "analysed",
        Bytecode::Eof(_) => "eof",
    };
    g.bench_function(format!("transact/{state}"), |b| b.iter(|| evm.transact().unwrap()));
}

fn bench_eval(g: &mut BenchmarkGroup<criterion::measurement::WallTime>, evm: &mut Evm<'static, (), BenchmarkDB>) {
    g.bench_function("eval", |b| {
        let contract = Contract {
            input: evm.context.evm.env.tx.data.clone(),
            bytecode: to_analysed(evm.context.evm.db.0.clone()),
            ..Default::default()
        };
        let mut shared_memory = SharedMemory::new();
        let mut host = DummyHost::new(*evm.context.evm.env.clone());
        let instruction_table = make_instruction_table::<DummyHost, BerlinSpec>();
        b.iter(move || {
            let temp = std::mem::replace(&mut shared_memory, EMPTY_SHARED_MEMORY);
            let mut interpreter = Interpreter::new(contract.clone(), u64::MAX, false);
            let res = interpreter.run(temp, &instruction_table, &mut host);
            shared_memory = interpreter.take_memory();
            host.clear();
            res
        })
    });
}

fn bytecode(s: &str) -> Bytecode {
    to_analysed(Bytecode::new_raw(hex::decode(s).unwrap().into()))
}

criterion_group!(benches, analysis, snailtracer, transfer);
criterion_main!(benches);

const ANALYSIS: &str = "6060604052341561000f57600080fd5b604051610dd138038061...";
const SNAILTRACER: &str = "608060405234801561001057600080fd5b5060043610...";
