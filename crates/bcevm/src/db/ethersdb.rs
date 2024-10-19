use std::sync::Arc;
use ethers_core::types::{Block, BlockId, TxHash, H160 as eH160, H256, U64 as eU64};
use ethers_providers::Middleware;
use tokio::runtime::{Builder, Handle, RuntimeFlavor};
use crate::primitives::{AccountInfo, Address, Bytecode, B256, KECCAK_EMPTY, U256};
use crate::{Database, DatabaseRef};

#[derive(Debug, Clone)]
pub struct EthersDB<M: Middleware> {
    client: Arc<M>,
    block_number: Option<BlockId>,
}

impl<M: Middleware> EthersDB<M> {
    pub fn new(client: Arc<M>, block_number: Option<BlockId>) -> Option<Self> {
        let block_number = block_number.or_else(|| 
            Some(BlockId::from(Self::block_on(client.get_block_number()).ok()?))
        );

        Some(Self { client, block_number })
    }

    #[inline]
    fn block_on<F: core::future::Future + Send>(f: F) -> F::Output
    where
        F::Output: Send,
    {
        match Handle::try_current() {
            Ok(handle) => match handle.runtime_flavor() {
                RuntimeFlavor::CurrentThread => std::thread::scope(|s| 
                    s.spawn(|| Builder::new_current_thread().enable_all().build().unwrap().block_on(f))
                        .join()
                        .unwrap()
                ),
                _ => tokio::task::block_in_place(move || handle.block_on(f)),
            },
            Err(_) => Builder::new_current_thread().enable_all().build().unwrap().block_on(f),
        }
    }

    #[inline]
    pub fn set_block_number(&mut self, block_number: BlockId) {
        self.block_number = Some(block_number);
    }
}

impl<M: Middleware> DatabaseRef for EthersDB<M> {
    type Error = M::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let add = eH160::from(address.0.0);

        let (nonce, balance, code) = Self::block_on(async {
            let nonce = self.client.get_transaction_count(add, self.block_number);
            let balance = self.client.get_balance(add, self.block_number);
            let code = self.client.get_code(add, self.block_number);
            tokio::join!(nonce, balance, code)
        });

        let balance = U256::from_limbs(balance?.0);
        let nonce = nonce?.as_u64();
        let bytecode = Bytecode::new_raw(code?.0.into());
        let code_hash = bytecode.hash_slow();
        
        Ok(Some(AccountInfo::new(balance, nonce, code_hash, bytecode)))
    }

    fn code_by_hash_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        panic!("Should not be called. Code is already loaded");
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let add = eH160::from(address.0.0);
        let index = H256::from(index.to_be_bytes());
        let slot_value: H256 = Self::block_on(self.client.get_storage_at(add, index, self.block_number))?;
        Ok(U256::from_be_bytes(slot_value.to_fixed_bytes()))
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        if number > U256::from(u64::MAX) {
            return Ok(KECCAK_EMPTY);
        }
        let number = eU64::from(u64::try_from(number).unwrap());
        let block: Option<Block<TxHash>> = Self::block_on(self.client.get_block(BlockId::from(number)))?;
        Ok(B256::new(block.unwrap().hash.unwrap().0))
    }
}

impl<M: Middleware> Database for EthersDB<M> {
    type Error = M::Error;

    #[inline]
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        <Self as DatabaseRef>::basic_ref(self, address)
    }

    #[inline]
    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        <Self as DatabaseRef>::code_by_hash_ref(self, code_hash)
    }

    #[inline]
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        <Self as DatabaseRef>::storage_ref(self, address, index)
    }

    #[inline]
    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        <Self as DatabaseRef>::block_hash_ref(self, number)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ethers_providers::{Http, Provider};

    #[test]
    fn _can_get_basic() {
        let client = Arc::new(Provider::<Http>::try_from("https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27").unwrap());
        let ethersdb = EthersDB::new(Arc::clone(&client), Some(BlockId::from(16148323))).unwrap();

        let address: Address = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".parse::<eH160>().unwrap().as_fixed_bytes().into();
        let acc_info = ethersdb.basic_ref(address).unwrap().unwrap();

        assert!(acc_info.exists());
    }
}
