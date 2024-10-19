use super::{DatabaseCommit, DatabaseRef, EmptyDB};
use crate::primitives::{
    hash_map::Entry, Account, AccountInfo, Address, Bytecode, HashMap, Log, B256, KECCAK_EMPTY,
    U256,
};
use crate::Database;
use core::convert::Infallible;

pub type InMemoryDB = CacheDB<EmptyDB>;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CacheDB<ExtDB> {
    pub accounts: HashMap<Address, DbAccount>,
    pub contracts: HashMap<B256, Bytecode>,
    pub logs: Vec<Log>,
    pub block_hashes: HashMap<U256, B256>,
    pub db: ExtDB,
}

impl<ExtDB: Default> Default for CacheDB<ExtDB> {
    fn default() -> Self {
        Self::new(ExtDB::default())
    }
}

impl<ExtDB> CacheDB<ExtDB> {
    pub fn new(db: ExtDB) -> Self {
        let mut contracts = HashMap::new();
        contracts.insert(KECCAK_EMPTY, Bytecode::default());
        contracts.insert(B256::ZERO, Bytecode::default());
        Self {
            accounts: HashMap::new(),
            contracts,
            logs: Vec::new(),
            block_hashes: HashMap::new(),
            db,
        }
    }

    pub fn insert_contract(&mut self, account: &mut AccountInfo) {
        if let Some(code) = &account.code {
            if !code.is_empty() {
                if account.code_hash == KECCAK_EMPTY {
                    account.code_hash = code.hash_slow();
                }
                self.contracts.entry(account.code_hash).or_insert_with(|| code.clone());
            }
        }
        if account.code_hash == B256::ZERO {
            account.code_hash = KECCAK_EMPTY;
        }
    }

    pub fn insert_account_info(&mut self, address: Address, mut info: AccountInfo) {
        self.insert_contract(&mut info);
        self.accounts.entry(address).or_default().info = info;
    }
}

impl<ExtDB: DatabaseRef> CacheDB<ExtDB> {
    pub fn load_account(&mut self, address: Address) -> Result<&mut DbAccount, ExtDB::Error> {
        Ok(self.accounts.entry(address).or_insert_with(|| {
            self.db.basic_ref(address)
                .transpose()
                .unwrap_or_else(|| DbAccount::new_not_existing())
        }))
    }

    pub fn insert_account_storage(&mut self, address: Address, slot: U256, value: U256) -> Result<(), ExtDB::Error> {
        self.load_account(address)?.storage.insert(slot, value);
        Ok(())
    }

    pub fn replace_account_storage(&mut self, address: Address, storage: HashMap<U256, U256>) -> Result<(), ExtDB::Error> {
        let account = self.load_account(address)?;
        account.account_state = AccountState::StorageCleared;
        account.storage = storage;
        Ok(())
    }
}

impl<ExtDB> DatabaseCommit for CacheDB<ExtDB> {
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        for (address, mut account) in changes {
            if !account.is_touched() {
                continue;
            }
            let db_account = self.accounts.entry(address).or_default();
            if account.is_selfdestructed() {
                *db_account = DbAccount::new_not_existing();
                continue;
            }
            self.insert_contract(&mut account.info);
            db_account.info = account.info;
            db_account.account_state = if account.is_created() {
                db_account.storage.clear();
                AccountState::StorageCleared
            } else if db_account.account_state.is_storage_cleared() {
                AccountState::StorageCleared
            } else {
                AccountState::Touched
            };
            db_account.storage.extend(account.storage.into_iter().map(|(k, v)| (k, v.present_value())));
        }
    }
}

impl<ExtDB: DatabaseRef> Database for CacheDB<ExtDB> {
    type Error = ExtDB::Error;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(self.load_account(address)?.info())
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.contracts.entry(code_hash).or_insert_with(|| self.db.code_by_hash_ref(code_hash).unwrap()).clone())
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        let account = self.load_account(address)?;
        Ok(*account.storage.entry(index).or_insert_with(|| {
            if account.account_state.is_storage_cleared() {
                U256::ZERO
            } else {
                self.db.storage_ref(address, index).unwrap()
            }
        }))
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        Ok(*self.block_hashes.entry(number).or_insert_with(|| self.db.block_hash_ref(number).unwrap()))
    }
}

impl<ExtDB: DatabaseRef> DatabaseRef for CacheDB<ExtDB> {
    type Error = ExtDB::Error;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(self.accounts.get(&address).map_or_else(|| self.db.basic_ref(address).unwrap(), |acc| acc.info()))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(self.contracts.get(&code_hash).cloned().unwrap_or_else(|| self.db.code_by_hash_ref(code_hash).unwrap()))
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        Ok(self.accounts.get(&address).map_or_else(
            || self.db.storage_ref(address, index).unwrap(),
            |acc| acc.storage.get(&index).cloned().unwrap_or_else(|| {
                if acc.account_state.is_storage_cleared() {
                    U256::ZERO
                } else {
                    self.db.storage_ref(address, index).unwrap()
                }
            })
        ))
    }

    fn block_hash_ref(&self, number: U256) -> Result<B256, Self::Error> {
        Ok(*self.block_hashes.get(&number).unwrap_or_else(|| &self.db.block_hash_ref(number).unwrap()))
    }
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DbAccount {
    pub info: AccountInfo,
    pub account_state: AccountState,
    pub storage: HashMap<U256, U256>,
}

impl DbAccount {
    pub fn new_not_existing() -> Self {
        Self {
            account_state: AccountState::NotExisting,
            ..Default::default()
        }
    }

    pub fn info(&self) -> Option<AccountInfo> {
        (!matches!(self.account_state, AccountState::NotExisting)).then(|| self.info.clone())
    }
}

impl From<Option<AccountInfo>> for DbAccount {
    fn from(from: Option<AccountInfo>) -> Self {
        from.map(Self::from).unwrap_or_else(Self::new_not_existing)
    }
}

impl From<AccountInfo> for DbAccount {
    fn from(info: AccountInfo) -> Self {
        Self {
            info,
            account_state: AccountState::None,
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AccountState {
    NotExisting,
    Touched,
    StorageCleared,
    #[default]
    None,
}

impl AccountState {
    pub fn is_storage_cleared(&self) -> bool {
        matches!(self, AccountState::StorageCleared)
    }
}

#[derive(Debug, Default, Clone)]
pub struct BenchmarkDB(pub Bytecode, B256);

impl BenchmarkDB {
    pub fn new_bytecode(bytecode: Bytecode) -> Self {
        Self(bytecode, bytecode.hash_slow())
    }
}

impl Database for BenchmarkDB {
    type Error = Infallible;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        Ok(match address {
            Address::ZERO => Some(AccountInfo {
                nonce: 1,
                balance: U256::from(10000000),
                code: Some(self.0.clone()),
                code_hash: self.1,
            }),
            _ if address == Address::with_last_byte(1) => Some(AccountInfo {
                nonce: 0,
                balance: U256::from(10000000),
                code: None,
                code_hash: KECCAK_EMPTY,
            }),
            _ => None,
        })
    }

    fn code_by_hash(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(Bytecode::default())
    }

    fn storage(&mut self, _address: Address, _index: U256) -> Result<U256, Self::Error> {
        Ok(U256::default())
    }

    fn block_hash(&mut self, _number: U256) -> Result<B256, Self::Error> {
        Ok(B256::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_account_storage() {
        let account = Address::with_last_byte(42);
        let nonce = 42;
        let mut init_state = CacheDB::new(EmptyDB::default());
        init_state.insert_account_info(account, AccountInfo { nonce, ..Default::default() });

        let (key, value) = (U256::from(123), U256::from(456));
        let mut new_state = CacheDB::new(init_state);
        new_state.insert_account_storage(account, key, value).unwrap();

        assert_eq!(new_state.basic(account).unwrap().unwrap().nonce, nonce);
        assert_eq!(new_state.storage(account, key), Ok(value));
    }

    #[test]
    fn test_replace_account_storage() {
        let account = Address::with_last_byte(42);
        let nonce = 42;
        let mut init_state = CacheDB::new(EmptyDB::default());
        init_state.insert_account_info(account, AccountInfo { nonce, ..Default::default() });

        let (key0, value0) = (U256::from(123), U256::from(456));
        let (key1, value1) = (U256::from(789), U256::from(999));
        init_state.insert_account_storage(account, key0, value0).unwrap();

        let mut new_state = CacheDB::new(init_state);
        new_state.replace_account_storage(account, [(key1, value1)].into()).unwrap();

        assert_eq!(new_state.basic(account).unwrap().unwrap().nonce, nonce);
        assert_eq!(new_state.storage(account, key0), Ok(U256::ZERO));
        assert_eq!(new_state.storage(account, key1), Ok(value1));
    }
}
