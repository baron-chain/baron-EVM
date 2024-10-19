use super::{AccountRevert, BundleAccount, StorageWithOriginalValues};
use crate::db::AccountStatus;
use bcevm_interpreter::primitives::{hash_map, AccountInfo, Bytecode, B256, I256, U256};

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct TransitionAccount {
    pub info: Option<AccountInfo>,
    pub status: AccountStatus,
    pub previous_info: Option<AccountInfo>,
    pub previous_status: AccountStatus,
    pub storage: StorageWithOriginalValues,
    pub storage_was_destroyed: bool,
}

impl TransitionAccount {
    pub fn new_empty_eip161(storage: StorageWithOriginalValues) -> Self {
        Self {
            info: Some(AccountInfo::default()),
            status: AccountStatus::InMemoryChange,
            previous_info: None,
            previous_status: AccountStatus::LoadedNotExisting,
            storage,
            storage_was_destroyed: false,
        }
    }

    pub fn has_new_contract(&self) -> Option<(B256, &Bytecode)> {
        let (present_new_codehash, previous_codehash) = match (&self.info, &self.previous_info) {
            (Some(info), Some(prev_info)) if info.code_hash != prev_info.code_hash => 
                (Some(&info.code_hash), Some(&prev_info.code_hash)),
            (Some(info), None) => (Some(&info.code_hash), None),
            _ => return None,
        };

        if present_new_codehash != previous_codehash {
            self.info.as_ref().and_then(|info| info.code.as_ref().map(|c| (info.code_hash, c)))
        } else {
            None
        }
    }

    pub fn previous_balance(&self) -> U256 {
        self.previous_info.as_ref().map_or(U256::ZERO, |info| info.balance)
    }

    pub fn current_balance(&self) -> U256 {
        self.info.as_ref().map_or(U256::ZERO, |info| info.balance)
    }

    pub fn balance_delta(&self) -> Option<I256> {
        let previous_balance = self.previous_balance();
        let current_balance = self.current_balance();
        let delta = I256::try_from(previous_balance.abs_diff(current_balance)).ok()?;
        if current_balance >= previous_balance {
            Some(delta)
        } else {
            delta.checked_neg()
        }
    }

    pub fn update(&mut self, other: Self) {
        self.info = other.info;
        self.status = other.status;

        if matches!(other.status, AccountStatus::Destroyed | AccountStatus::DestroyedAgain) {
            self.storage = other.storage;
            self.storage_was_destroyed = true;
        } else {
            for (key, slot) in other.storage {
                match self.storage.entry(key) {
                    hash_map::Entry::Vacant(entry) => { entry.insert(slot); }
                    hash_map::Entry::Occupied(mut entry) => {
                        let value = entry.get_mut();
                        if value.original_value() == slot.present_value() {
                            entry.remove();
                        } else {
                            value.present_value = slot.present_value;
                        }
                    }
                }
            }
        }
    }

    pub fn create_revert(self) -> Option<AccountRevert> {
        let mut previous_account = self.original_bundle_account();
        previous_account.update_and_create_revert(self)
    }

    pub fn present_bundle_account(&self) -> BundleAccount {
        BundleAccount {
            info: self.info.clone(),
            original_info: self.previous_info.clone(),
            storage: self.storage.clone(),
            status: self.status,
        }
    }

    fn original_bundle_account(&self) -> BundleAccount {
        BundleAccount {
            info: self.previous_info.clone(),
            original_info: self.previous_info.clone(),
            storage: StorageWithOriginalValues::new(),
            status: self.previous_status,
        }
    }
}
