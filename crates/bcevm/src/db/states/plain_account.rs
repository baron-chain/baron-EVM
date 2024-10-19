use bcevm_interpreter::primitives::{AccountInfo, HashMap, StorageSlot, U256};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PlainAccount {
    pub info: AccountInfo,
    pub storage: PlainStorage,
}

impl PlainAccount {
    pub fn new_empty_with_storage(storage: PlainStorage) -> Self {
        Self {
            info: AccountInfo::default(),
            storage,
        }
    }

    pub fn into_components(self) -> (AccountInfo, PlainStorage) {
        (self.info, self.storage)
    }
}

pub type StorageWithOriginalValues = HashMap<U256, StorageSlot>;
pub type PlainStorage = HashMap<U256, U256>;

impl From<AccountInfo> for PlainAccount {
    fn from(info: AccountInfo) -> Self {
        Self {
            info,
            storage: HashMap::new(),
        }
    }
}
