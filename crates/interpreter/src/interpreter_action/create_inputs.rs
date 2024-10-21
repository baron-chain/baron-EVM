pub use crate::primitives::CreateScheme;
use crate::primitives::{Address, Bytes, TransactTo, TxEnv, U256};
use std::boxed::Box;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateInputs {
    pub caller: Address,
    pub scheme: CreateScheme,
    pub value: U256,
    pub init_code: Bytes,
    pub gas_limit: u64,
}

impl CreateInputs {
    pub fn new(tx_env: &TxEnv, gas_limit: u64) -> Option<Self> {
        match tx_env.transact_to {
            TransactTo::Create => Some(Self {
                caller: tx_env.caller,
                scheme: CreateScheme::Create,
                value: tx_env.value,
                init_code: tx_env.data.clone(),
                gas_limit,
            }),
            _ => None,
        }
    }

    pub fn new_boxed(tx_env: &TxEnv, gas_limit: u64) -> Option<Box<Self>> {
        Self::new(tx_env, gas_limit).map(Box::new)
    }

    pub fn created_address(&self, nonce: u64) -> Address {
        match self.scheme {
            CreateScheme::Create => self.caller.create(nonce),
            CreateScheme::Create2 { salt } => self.caller.create2_from_code(salt.to_be_bytes(), &self.init_code),
        }
    }
}
