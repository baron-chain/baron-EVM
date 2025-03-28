use core::{convert::Infallible, fmt, marker::PhantomData};
use bcevm_interpreter::primitives::{
    db::{Database, DatabaseRef},
    keccak256, AccountInfo, Address, Bytecode, B256, U256,
};

pub type EmptyDB = EmptyDBTyped<Infallible>;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct EmptyDBTyped<E> {
    _phantom: PhantomData<E>,
}

impl<E> fmt::Debug for EmptyDBTyped<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EmptyDB").finish_non_exhaustive()
    }
}

impl<E> EmptyDBTyped<E> {
    pub fn new() -> Self {
        Self { _phantom: PhantomData }
    }
}

impl<E> Database for EmptyDBTyped<E> {
    type Error = E;

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

impl<E> DatabaseRef for EmptyDBTyped<E> {
    type Error = E;

    #[inline]
    fn basic_ref(&self, _address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(None)
    }

    #[inline]
    fn code_by_hash_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(Bytecode::default())
    }

    #[inline]
    fn storage_ref(&self, _address: Address, _index: U256) -> Result<U256, Self::Error> {
        Ok(U256::default())
    }

    #[inline]
    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        Ok(keccak256(number.to_string().as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::b256;

    #[test]
    fn conform_block_hash_calculation() {
        let db = EmptyDB::new();
        assert_eq!(
            db.block_hash_ref(U256::from(0)),
            Ok(b256!("044852b2a670ade5407e78fb2863c51de9fcb96542a07186fe3aeda6bb8a116d"))
        );
        assert_eq!(
            db.block_hash_ref(U256::from(1)),
            Ok(b256!("c89efdaa54c0f20c7adf612882df0950f5a951637e0307cdcb4c672f298b8bc6"))
        );
        assert_eq!(
            db.block_hash_ref(U256::from(100)),
            Ok(b256!("8c18210df0d9514f2d2e5d8ca7c100978219ee80d3968ad850ab5ead208287b3"))
        );
    }
}
