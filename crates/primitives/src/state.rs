use crate::{Address, Bytecode, HashMap, B256, KECCAK_EMPTY, U256};
use bitflags::bitflags;
use core::hash::{Hash, Hasher};

pub type State = HashMap<Address, Account>;
pub type TransientStorage = HashMap<(Address, U256), U256>;
pub type Storage = HashMap<U256, StorageSlot>;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Account {
    pub info: AccountInfo,
    pub storage: Storage,
    pub status: AccountStatus,
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[cfg_attr(feature = "serde", serde(transparent))]
    pub struct AccountStatus: u8 {
        const Loaded = 0b00000000;
        const Created = 0b00000001;
        const SelfDestructed = 0b00000010;
        const Touched = 0b00000100;
        const LoadedAsNotExisting = 0b00001000;
    }
}

impl Default for AccountStatus {
    fn default() -> Self {
        Self::Loaded
    }
}

impl Account {
    pub fn new_not_existing() -> Self {
        Self {
            info: AccountInfo::default(),
            storage: HashMap::new(),
            status: AccountStatus::LoadedAsNotExisting,
        }
    }

    pub fn mark_selfdestruct(&mut self) { self.status |= AccountStatus::SelfDestructed; }
    pub fn unmark_selfdestruct(&mut self) { self.status -= AccountStatus::SelfDestructed; }
    pub fn is_selfdestructed(&self) -> bool { self.status.contains(AccountStatus::SelfDestructed) }
    pub fn mark_touch(&mut self) { self.status |= AccountStatus::Touched; }
    pub fn unmark_touch(&mut self) { self.status -= AccountStatus::Touched; }
    pub fn is_touched(&self) -> bool { self.status.contains(AccountStatus::Touched) }
    pub fn mark_created(&mut self) { self.status |= AccountStatus::Created; }
    pub fn unmark_created(&mut self) { self.status -= AccountStatus::Created; }
    pub fn is_loaded_as_not_existing(&self) -> bool { self.status.contains(AccountStatus::LoadedAsNotExisting) }
    pub fn is_created(&self) -> bool { self.status.contains(AccountStatus::Created) }
    pub fn is_empty(&self) -> bool { self.info.is_empty() }

    pub fn changed_storage_slots(&self) -> impl Iterator<Item = (&U256, &StorageSlot)> {
        self.storage.iter().filter(|(_, slot)| slot.is_changed())
    }
}

impl From<AccountInfo> for Account {
    fn from(info: AccountInfo) -> Self {
        Self {
            info,
            storage: HashMap::new(),
            status: AccountStatus::Loaded,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StorageSlot {
    pub previous_or_original_value: U256,
    pub present_value: U256,
}

impl StorageSlot {
    pub fn new(original: U256) -> Self {
        Self { previous_or_original_value: original, present_value: original }
    }

    pub fn new_changed(previous_or_original_value: U256, present_value: U256) -> Self {
        Self { previous_or_original_value, present_value }
    }

    pub fn is_changed(&self) -> bool { self.previous_or_original_value != self.present_value }
    pub fn original_value(&self) -> U256 { self.previous_or_original_value }
    pub fn present_value(&self) -> U256 { self.present_value }
}

#[derive(Clone, Debug, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct AccountInfo {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: B256,
    pub code: Option<Bytecode>,
}

impl Default for AccountInfo {
    fn default() -> Self {
        Self {
            balance: U256::ZERO,
            code_hash: KECCAK_EMPTY,
            code: Some(Bytecode::default()),
            nonce: 0,
        }
    }
}

impl PartialEq for AccountInfo {
    fn eq(&self, other: &Self) -> bool {
        self.balance == other.balance && self.nonce == other.nonce && self.code_hash == other.code_hash
    }
}

impl Hash for AccountInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.balance.hash(state);
        self.nonce.hash(state);
        self.code_hash.hash(state);
    }
}

impl AccountInfo {
    pub fn new(balance: U256, nonce: u64, code_hash: B256, code: Bytecode) -> Self {
        Self { balance, nonce, code: Some(code), code_hash }
    }

    pub fn without_code(mut self) -> Self {
        self.take_bytecode();
        self
    }

    pub fn is_empty(&self) -> bool {
        (self.is_empty_code_hash() || self.code_hash == B256::ZERO) && self.balance.is_zero() && self.nonce == 0
    }

    pub fn exists(&self) -> bool { !self.is_empty() }
    pub fn has_no_code_and_nonce(&self) -> bool { self.is_empty_code_hash() && self.nonce == 0 }
    pub fn code_hash(&self) -> B256 { self.code_hash }
    pub fn is_empty_code_hash(&self) -> bool { self.code_hash == KECCAK_EMPTY }
    pub fn take_bytecode(&mut self) -> Option<Bytecode> { self.code.take() }

    pub fn from_balance(balance: U256) -> Self {
        Self { balance, ..Default::default() }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Account, KECCAK_EMPTY, U256};

    #[test]
    fn account_is_empty() {
        let mut account = Account::default();
        assert!(account.is_empty());

        account.info.balance = U256::from(1);
        assert!(!account.is_empty());
        account.info.balance = U256::ZERO;
        assert!(account.is_empty());

        account.info.nonce = 1;
        assert!(!account.is_empty());
        account.info.nonce = 0;
        assert!(account.is_empty());

        account.info.code_hash = [1; 32].into();
        assert!(!account.is_empty());
        account.info.code_hash = [0; 32].into();
        assert!(account.is_empty());
        account.info.code_hash = KECCAK_EMPTY;
        assert!(account.is_empty());
    }

    #[test]
    fn account_state() {
        let mut account = Account::default();
        assert!(!account.is_touched() && !account.is_selfdestructed());

        account.mark_touch();
        assert!(account.is_touched() && !account.is_selfdestructed());

        account.mark_selfdestruct();
        assert!(account.is_touched() && account.is_selfdestructed());

        account.unmark_selfdestruct();
        assert!(account.is_touched() && !account.is_selfdestructed());
    }
}
