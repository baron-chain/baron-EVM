use super::{
    plain_account::PlainStorage, transition_account::TransitionAccount, CacheAccount, PlainAccount,
};
use bcevm_interpreter::primitives::{
    Account, AccountInfo, Address, Bytecode, HashMap, State as EVMState, B256,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CacheState {
    pub accounts: HashMap<Address, CacheAccount>,
    pub contracts: HashMap<B256, Bytecode>,
    pub has_state_clear: bool,
}

impl Default for CacheState {
    fn default() -> Self {
        Self::new(true)
    }
}

impl CacheState {
    pub fn new(has_state_clear: bool) -> Self {
        Self {
            accounts: HashMap::default(),
            contracts: HashMap::default(),
            has_state_clear,
        }
    }

    pub fn set_state_clear_flag(&mut self, has_state_clear: bool) {
        self.has_state_clear = has_state_clear;
    }

    pub fn trie_account(&self) -> impl IntoIterator<Item = (Address, &PlainAccount)> {
        self.accounts.iter().filter_map(|(address, account)| {
            account.account.as_ref().map(|plain_acc| (*address, plain_acc))
        })
    }

    pub fn insert_not_existing(&mut self, address: Address) {
        self.accounts.insert(address, CacheAccount::new_loaded_not_existing());
    }

    pub fn insert_account(&mut self, address: Address, info: AccountInfo) {
        let account = if !info.is_empty() {
            CacheAccount::new_loaded(info, HashMap::default())
        } else {
            CacheAccount::new_loaded_empty_eip161(HashMap::default())
        };
        self.accounts.insert(address, account);
    }

    pub fn insert_account_with_storage(
        &mut self,
        address: Address,
        info: AccountInfo,
        storage: PlainStorage,
    ) {
        let account = if !info.is_empty() {
            CacheAccount::new_loaded(info, storage)
        } else {
            CacheAccount::new_loaded_empty_eip161(storage)
        };
        self.accounts.insert(address, account);
    }

    pub fn apply_evm_state(&mut self, evm_state: EVMState) -> Vec<(Address, TransitionAccount)> {
        evm_state
            .into_iter()
            .filter_map(|(address, account)| {
                self.apply_account_state(address, account)
                    .map(|transition| (address, transition))
            })
            .collect()
    }

    fn apply_account_state(
        &mut self,
        address: Address,
        account: Account,
    ) -> Option<TransitionAccount> {
        if !account.is_touched() {
            return None;
        }

        let this_account = self.accounts.get_mut(&address).expect("All accounts should be present inside cache");

        if account.is_selfdestructed() {
            return this_account.selfdestruct();
        }

        if account.is_created() {
            return Some(this_account.newly_created(account.info, account.storage));
        }

        if account.is_empty() {
            if self.has_state_clear {
                this_account.touch_empty_eip161()
            } else {
                this_account.touch_create_pre_eip161(account.storage)
            }
        } else {
            Some(this_account.change(account.info, account.storage))
        }
    }
}
