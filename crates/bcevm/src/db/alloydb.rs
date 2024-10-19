use crate::{
    db::{Database, DatabaseRef},
    primitives::{AccountInfo, Address, Bytecode, B256, KECCAK_EMPTY, U256},
};
use alloy_provider::{Network, Provider};
use alloy_rpc_types::BlockId;
use alloy_transport::{Transport, TransportError};
use tokio::runtime::{Builder, Handle};

#[derive(Debug, Clone)]
pub struct AlloyDB<T: Transport + Clone, N: Network, P: Provider<T, N>> {
    provider: P,
    block_number: Option<BlockId>,
    _marker: std::marker::PhantomData<fn() -> (T, N)>,
}

impl<T: Transport + Clone, N: Network, P: Provider<T, N>> AlloyDB<T, N, P> {
    pub fn new(provider: P, block_number: Option<BlockId>) -> Self {
        Self { provider, block_number, _marker: std::marker::PhantomData }
    }

    fn block_on<F: std::future::Future + Send>(f: F) -> F::Output
    where
        F::Output: Send,
    {
        match Handle::try_current() {
            Ok(handle) => match handle.runtime_flavor() {
                tokio::runtime::RuntimeFlavor::CurrentThread => std::thread::scope(move |s| {
                    s.spawn(move || Builder::new_current_thread().enable_all().build().unwrap().block_on(f))
                        .join()
                        .unwrap()
                }),
                _ => tokio::task::block_in_place(move || handle.block_on(f)),
            },
            Err(_) => Builder::new_current_thread().enable_all().build().unwrap().block_on(f),
        }
    }

    pub fn set_block_number(&mut self, block_number: Option<BlockId>) {
        self.block_number = block_number;
    }
}

impl<T: Transport + Clone, N: Network, P: Provider<T, N>> DatabaseRef for AlloyDB<T, N, P> {
    type Error = TransportError;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let (nonce, balance, code) = Self::block_on(async {
            let nonce = self.provider.get_transaction_count(address, self.block_number);
            let balance = self.provider.get_balance(address, self.block_number);
            let code = self.provider.get_code_at(address, self.block_number.unwrap_or_default());
            tokio::join!(nonce, balance, code)
        });

        let balance = balance?;
        let code = Bytecode::new_raw(code?.0.into());
        let code_hash = code.hash_slow();
        let nonce = nonce?;

        Ok(Some(AccountInfo::new(balance, nonce.to::<u64>(), code_hash, code)))
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        if number > U256::from(u64::MAX) {
            return Ok(KECCAK_EMPTY);
        }

        let block = Self::block_on(self.provider.get_block_by_number(number.to::<u64>().into(), false))?;
        Ok(B256::new(*block.unwrap().header.hash.unwrap()))
    }

    fn code_by_hash_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        panic!("This should not be called, as the code is already loaded");
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Self::block_on(self.provider.get_storage_at(address, index, self.block_number))
    }
}

impl<T: Transport + Clone, N: Network, P: Provider<T, N>> Database for AlloyDB<T, N, P> {
    type Error = TransportError;

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
    use alloy_provider::ProviderBuilder;

    #[test]
    fn can_get_basic() {
        let client = ProviderBuilder::new()
            .on_reqwest_http("https://mainnet.infura.io/v3/c60b0bb42f8a4c6481ecd229eddaca27".parse().unwrap())
            .unwrap();
        let alloydb = AlloyDB::new(client, Some(BlockId::from(16148323)));

        let address: Address = "0x0d4a11d5EEaaC28EC3F61d100daF4d40471f1852".parse().unwrap();

        let acc_info = alloydb.basic_ref(address).unwrap().unwrap();
        assert!(acc_info.exists());
    }
}
