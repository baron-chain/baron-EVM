use ethers_core::types::BlockId;
use ethers_providers::{Http, Middleware, Provider};
use indicatif::ProgressBar;
use bcevm::{
    db::{CacheDB, EthersDB, StateBuilder},
    inspectors::TracerEip3155,
    primitives::{Address, TransactTo, U256},
    inspector_handle_register, Evm,
};
use std::{
    fs::{self, OpenOptions},
    io::{BufWriter, Write},
    sync::{Arc, Mutex},
    time::Instant,
};

macro_rules! local_fill {
    ($left:expr, $right:expr, $fun:expr) => {
        if let Some(right) = $right {
            $left = $fun(right.0)
        }
    };
    ($left:expr, $right:expr) => {
        if let Some(right) = $right {
            $left = Address::from(right.as_fixed_bytes())
        }
    };
}

struct FlushWriter {
    writer: Arc<Mutex<BufWriter<fs::File>>>,
}

impl FlushWriter {
    fn new(writer: Arc<Mutex<BufWriter<fs::File>>>) -> Self {
        Self { writer }
    }
}

impl Write for FlushWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.writer.lock().unwrap().write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.lock().unwrap().flush()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Arc::new(Provider::<Http>::try_from(
        "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27",
    )?);

    let chain_id: u64 = 1;
    let block_number = 10889447;

    let block = client.get_block_with_txs(block_number).await?.ok_or_else(|| anyhow::anyhow!("Block not found"))?;
    println!("Fetched block number: {}", block.number.unwrap().0[0]);

    let prev_id: BlockId = (block_number - 1).into();
    let state_db = EthersDB::new(Arc::clone(&client), Some(prev_id))?;
    let cache_db = CacheDB::new(state_db);
    let mut state = StateBuilder::new_with_database(cache_db).build();

    let mut evm = Evm::builder()
        .with_db(&mut state)
        .with_external_context(TracerEip3155::new(Box::new(std::io::stdout())))
        .modify_block_env(|b| {
            if let Some(number) = block.number {
                b.number = U256::from(number.0[0]);
            }
            local_fill!(b.coinbase, block.author);
            local_fill!(b.timestamp, Some(block.timestamp), U256::from_limbs);
            local_fill!(b.difficulty, Some(block.difficulty), U256::from_limbs);
            local_fill!(b.gas_limit, Some(block.gas_limit), U256::from_limbs);
            if let Some(base_fee) = block.base_fee_per_gas {
                local_fill!(b.basefee, Some(base_fee), U256::from_limbs);
            }
        })
        .modify_cfg_env(|c| { c.chain_id = chain_id; })
        .append_handler_register(inspector_handle_register)
        .build();

    let txs = block.transactions.len();
    println!("Found {txs} transactions.");

    let console_bar = Arc::new(ProgressBar::new(txs as u64));
    let start = Instant::now();

    fs::create_dir_all("traces")?;

    for tx in block.transactions {
        evm = evm.modify()
            .modify_tx_env(|etx| {
                etx.caller = Address::from(tx.from.as_fixed_bytes());
                etx.gas_limit = tx.gas.as_u64();
                local_fill!(etx.gas_price, tx.gas_price, U256::from_limbs);
                local_fill!(etx.value, Some(tx.value), U256::from_limbs);
                etx.data = tx.input.0.into();
                let mut gas_priority_fee = U256::ZERO;
                local_fill!(gas_priority_fee, tx.max_priority_fee_per_gas, U256::from_limbs);
                etx.gas_priority_fee = Some(gas_priority_fee);
                etx.chain_id = Some(chain_id);
                etx.nonce = Some(tx.nonce.as_u64());
                etx.access_list = tx.access_list.map_or(Default::default(), |access_list| {
                    access_list.0.into_iter()
                        .map(|item| (
                            Address::from(item.address.as_fixed_bytes()),
                            item.storage_keys.into_iter().map(|h256| U256::from_le_bytes(h256.0)).collect()
                        ))
                        .collect()
                });
                etx.transact_to = tx.to.map_or(TransactTo::create(), |to_address| {
                    TransactTo::Call(Address::from(to_address.as_fixed_bytes()))
                });
            })
            .build();

        let tx_number = tx.transaction_index.unwrap().0[0];
        let file_name = format!("traces/{}.json", tx_number);
        let write = OpenOptions::new().write(true).create(true).truncate(true).open(file_name)?;
        let inner = Arc::new(Mutex::new(BufWriter::new(write)));
        let writer = FlushWriter::new(Arc::clone(&inner));

        evm.context.external.set_writer(Box::new(writer));
        if let Err(error) = evm.transact_commit() {
            eprintln!("Got error: {:?}", error);
        }

        inner.lock().unwrap().flush()?;
        console_bar.inc(1);
    }

    console_bar.finish_with_message("Finished all transactions.");
    println!("Finished execution. Total CPU time: {:.6}s", start.elapsed().as_secs_f64());

    Ok(())
}
