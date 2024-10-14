use crate::primitives::{Address, Bytecode, Env, Log, B256, U256};

mod dummy;
pub use dummy::DummyHost;

pub trait Host {
    fn env(&self) -> &Env;
    fn env_mut(&mut self) -> &mut Env;
    fn load_account(&mut self, address: Address) -> Option<LoadAccountResult>;
    fn block_hash(&mut self, number: U256) -> Option<B256>;
    fn balance(&mut self, address: Address) -> Option<(U256, bool)>;
    fn code(&mut self, address: Address) -> Option<(Bytecode, bool)>;
    fn code_hash(&mut self, address: Address) -> Option<(B256, bool)>;
    fn sload(&mut self, address: Address, index: U256) -> Option<(U256, bool)>;
    fn sstore(&mut self, address: Address, index: U256, value: U256) -> Option<SStoreResult>;
    fn tload(&mut self, address: Address, index: U256) -> U256;
    fn tstore(&mut self, address: Address, index: U256, value: U256);
    fn log(&mut self, log: Log);
    fn selfdestruct(&mut self, address: Address, target: Address) -> Option<SelfDestructResult>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SStoreResult {
    pub original_value: U256,
    pub present_value: U256,
    pub new_value: U256,
    pub is_cold: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LoadAccountResult {
    pub is_cold: bool,
    pub is_empty: bool,
}

#[derive(Default, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SelfDestructResult {
    pub had_value: bool,
    pub target_exists: bool,
    pub is_cold: bool,
    pub previously_destroyed: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_host<H: Host + ?Sized>() {}

    #[test]
    fn object_safety() {
        assert_host::<DummyHost>();
        assert_host::<dyn Host>();
    }
}
