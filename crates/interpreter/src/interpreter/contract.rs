use super::analysis::to_analysed;
use crate::{
    primitives::{Address, Bytecode, Bytes, Env, TransactTo, B256, U256},
    CallInputs,
};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Contract {
    pub input: Bytes,
    pub bytecode: Bytecode,
    pub hash: Option<B256>,
    pub target_address: Address,
    pub caller: Address,
    pub call_value: U256,
}

impl Contract {
    #[inline]
    pub fn new(
        input: Bytes,
        bytecode: Bytecode,
        hash: Option<B256>,
        target_address: Address,
        caller: Address,
        call_value: U256,
    ) -> Self {
        Self {
            input,
            bytecode: to_analysed(bytecode),
            hash,
            target_address,
            caller,
            call_value,
        }
    }

    #[inline]
    pub fn new_env(env: &Env, bytecode: Bytecode, hash: Option<B256>) -> Self {
        let contract_address = match env.tx.transact_to {
            TransactTo::Call(caller) => caller,
            TransactTo::Create => Address::ZERO,
        };
        Self::new(
            env.tx.data.clone(),
            bytecode,
            hash,
            contract_address,
            env.tx.caller,
            env.tx.value,
        )
    }

    #[inline]
    pub fn new_with_context(
        input: Bytes,
        bytecode: Bytecode,
        hash: Option<B256>,
        call_context: &CallInputs,
    ) -> Self {
        Self::new(
            input,
            bytecode,
            hash,
            call_context.target_address,
            call_context.caller,
            call_context.call_value(),
        )
    }

    #[inline]
    pub fn is_valid_jump(&self, pos: usize) -> bool {
        self.bytecode
            .legacy_jump_table()
            .map_or(false, |table| table.is_valid(pos))
    }
}
