use alloy_provider::{network::Ethereum, ProviderBuilder, RootProvider};
use alloy_sol_types::{sol, SolCall, SolValue};
use alloy_transport_http::Http;
use anyhow::{anyhow, Result};
use reqwest::Client;
use bcevm::{
    db::{AlloyDB, CacheDB},
    primitives::{
        address, keccak256, AccountInfo, Address, Bytes, ExecutionResult, Output, TransactTo, U256,
    },
    Evm,
};
use std::{ops::Div, sync::Arc};

type AlloyCacheDB = CacheDB<AlloyDB<Http<Client>, Ethereum, Arc<RootProvider<Http<Client>>>>>;

#[tokio::main]
async fn main() -> Result<()> {
    let client = Arc::new(ProviderBuilder::new()
        .on_reqwest_http("https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27".parse()?)
        .map_err(|e| anyhow!("Failed to create provider: {}", e))?);
    let mut cache_db = CacheDB::new(AlloyDB::new(client, None));

    let account = address!("18B06aaF27d44B756FCF16Ca20C1f183EB49111f");
    let weth = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
    let usdc = address!("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
    let usdc_weth_pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");

    let one_ether = U256::from(1_000_000_000_000_000_000u128);
    let hashed_acc_balance_slot = keccak256((account, U256::from(3)).abi_encode());
    cache_db.insert_account_storage(weth, hashed_acc_balance_slot.into(), one_ether)?;

    cache_db.insert_account_info(account, AccountInfo {
        nonce: 0, balance: one_ether,
        code_hash: keccak256(Bytes::new()),
        code: None,
    });

    println!("WETH balance before swap: {}", balance_of(weth, account, &mut cache_db)?);
    println!("USDC balance before swap: {}", balance_of(usdc, account, &mut cache_db)?);

    let (reserve0, reserve1) = get_reserves(usdc_weth_pair, &mut cache_db)?;
    let amount_in = one_ether.div(U256::from(10));
    let amount_out = get_amount_out(amount_in, reserve1, reserve0, &mut cache_db).await?;

    transfer(account, usdc_weth_pair, amount_in, weth, &mut cache_db)?;
    swap(account, usdc_weth_pair, account, amount_out, true, &mut cache_db)?;

    println!("WETH balance after swap: {}", balance_of(weth, account, &mut cache_db)?);
    println!("USDC balance after swap: {}", balance_of(usdc, account, &mut cache_db)?);

    Ok(())
}

fn balance_of(token: Address, account: Address, cache_db: &mut AlloyCacheDB) -> Result<U256> {
    sol! {
        function balanceOf(address account) public returns (uint256);
    }

    let encoded = balanceOfCall { account }.abi_encode();
    let result = execute_call(token, encoded, cache_db)?;
    U256::abi_decode(&result, false).map_err(|e| anyhow!("Failed to decode balance: {}", e))
}

async fn get_amount_out(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
    cache_db: &mut AlloyCacheDB,
) -> Result<U256> {
    sol! {
        function getAmountOut(uint amountIn, uint reserveIn, uint reserveOut) external pure returns (uint amountOut);
    }

    let encoded = getAmountOutCall { amountIn: amount_in, reserveIn: reserve_in, reserveOut: reserve_out }.abi_encode();
    let result = execute_call(address!("7a250d5630b4cf539739df2c5dacb4c659f2488d"), encoded, cache_db)?;
    U256::abi_decode(&result, false).map_err(|e| anyhow!("Failed to decode amount out: {}", e))
}

fn get_reserves(pair_address: Address, cache_db: &mut AlloyCacheDB) -> Result<(U256, U256)> {
    sol! {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }

    let encoded = getReservesCall {}.abi_encode();
    let result = execute_call(pair_address, encoded, cache_db)?;
    let (reserve0, reserve1, _) = <(U256, U256, u32)>::abi_decode(&result, false)
        .map_err(|e| anyhow!("Failed to decode reserves: {}", e))?;
    Ok((reserve0, reserve1))
}

fn swap(
    from: Address,
    pool_address: Address,
    target: Address,
    amount_out: U256,
    is_token0: bool,
    cache_db: &mut AlloyCacheDB,
) -> Result<()> {
    sol! {
        function swap(uint amount0Out, uint amount1Out, address target, bytes callback) external;
    }

    let (amount0_out, amount1_out) = if is_token0 { (amount_out, U256::ZERO) } else { (U256::ZERO, amount_out) };
    let encoded = swapCall { amount0Out: amount0_out, amount1Out: amount1_out, target, callback: Bytes::new() }.abi_encode();
    execute_commit(from, pool_address, encoded, cache_db)?;
    Ok(())
}

fn transfer(
    from: Address,
    to: Address,
    amount: U256,
    token: Address,
    cache_db: &mut AlloyCacheDB,
) -> Result<()> {
    sol! {
        function transfer(address to, uint amount) external returns (bool);
    }

    let encoded = transferCall { to, amount }.abi_encode();
    let result = execute_commit(from, token, encoded, cache_db)?;
    let success: bool = bool::abi_decode(&result, false)
        .map_err(|e| anyhow!("Failed to decode transfer result: {}", e))?;
    if !success {
        return Err(anyhow!("Transfer failed"));
    }
    Ok(())
}

fn execute_call(to: Address, data: Bytes, cache_db: &mut AlloyCacheDB) -> Result<Bytes> {
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = address!("0000000000000000000000000000000000000001");
            tx.transact_to = TransactTo::Call(to);
            tx.data = data;
            tx.value = U256::ZERO;
        })
        .build();

    match evm.transact()?.result {
        ExecutionResult::Success { output: Output::Call(value), .. } => Ok(value),
        result => Err(anyhow!("Execution failed: {:?}", result)),
    }
}

fn execute_commit(from: Address, to: Address, data: Bytes, cache_db: &mut AlloyCacheDB) -> Result<Bytes> {
    let mut evm = Evm::builder()
        .with_db(cache_db)
        .modify_tx_env(|tx| {
            tx.caller = from;
            tx.transact_to = TransactTo::Call(to);
            tx.data = data;
            tx.value = U256::ZERO;
        })
        .build();

    match evm.transact_commit()? {
        ExecutionResult::Success { output: Output::Call(value), .. } => Ok(value),
        result => Err(anyhow!("Execution failed: {:?}", result)),
    }
}
