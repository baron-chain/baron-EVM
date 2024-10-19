use super::RevertToSlot;
use bcevm_interpreter::primitives::{AccountInfo, Address, Bytecode, B256, U256};

#[derive(Clone, Debug, Default)]
pub struct StateChangeset {
    pub accounts: Vec<(Address, Option<AccountInfo>)>,
    pub storage: Vec<PlainStorageChangeset>,
    pub contracts: Vec<(B256, Bytecode)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlainStorageChangeset {
    pub address: Address,
    pub wipe_storage: bool,
    pub storage: Vec<(U256, U256)>,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct PlainStorageRevert {
    pub address: Address,
    pub wiped: bool,
    pub storage_revert: Vec<(U256, RevertToSlot)>,
}

#[derive(Clone, Debug, Default)]
pub struct PlainStateReverts {
    pub accounts: Vec<Vec<(Address, Option<AccountInfo>)>>,
    pub storage: Vec<Vec<PlainStorageRevert>>,
}

impl PlainStateReverts {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            accounts: Vec::with_capacity(capacity),
            storage: Vec::with_capacity(capacity),
        }
    }
}

pub type StorageRevert = Vec<Vec<(Address, bool, Vec<(U256, RevertToSlot)>)>>;
