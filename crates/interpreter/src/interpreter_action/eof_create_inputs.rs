use crate::primitives::{Address, Eof, U256};
use core::ops::Range;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EOFCreateInput {
    pub caller: Address,
    pub created_address: Address,
    pub value: U256,
    pub eof_init_code: Eof,
    pub gas_limit: u64,
    pub return_memory_range: Range<usize>,
}

impl EOFCreateInput {
    pub fn new(
        caller: Address,
        created_address: Address,
        value: U256,
        eof_init_code: Eof,
        gas_limit: u64,
        return_memory_range: Range<usize>,
    ) -> Self {
        Self {
            caller,
            created_address,
            value,
            eof_init_code,
            gas_limit,
            return_memory_range,
        }
    }
}
