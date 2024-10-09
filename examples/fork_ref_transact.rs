use ethers_contract::BaseContract;
use ethers_core::abi::parse_abi;
use ethers_providers::{Http, Provider};
use bcevm::{
    db::{CacheDB, EmptyDB, EthersDB},
    primitives::{address, ExecutionResult, Output, TransactTo, U256},
    Database, Evm,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Arc::new(Provider::<Http>::try_from(
        "https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27",
    )?);

    let slot = U256::from(8);
    let pool_address = address!("0d4a11d5EEaaC28EC3F61d100daF4d40471f1852");

    let abi = BaseContract::from(parse_abi(&[
        "function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)",
    ])?);

    let encoded = abi.encode("getReserves", ())?;

    let mut ethersdb = EthersDB::new(Arc::clone(&client), None)?;
    let acc_info = ethersdb.basic(pool_address)?.unwrap();
    let value = ethersdb.storage(pool_address, slot)?;

    let mut cache_db = CacheDB::new(EmptyDB::default());
    cache_db.insert_account_info(pool_address, acc_info);
    cache_db.insert_account_storage(pool_address, slot, value)?;

    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = address!("0000000000000000000000000000000000000000");
            tx.transact_to = TransactTo::Call(pool_address);
            tx.data = encoded.0.into();
            tx.value = U256::from(0);
        })
        .build();

    let ref_tx = evm.transact()?;
    let value = match ref_tx.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => value,
        result => anyhow::bail!("Execution failed: {result:?}"),
    };

    let (reserve0, reserve1, ts): (u128, u128, u32) = abi.decode_output("getReserves", value)?;

    println!("Reserve0: {reserve0:#?}");
    println!("Reserve1: {reserve1:#?}");
    println!("Timestamp: {ts:#?}");

    Ok(())
}
