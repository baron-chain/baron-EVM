use crate::primitives::{Address, Bytes, TransactTo, TxEnv, U256};
use core::ops::Range;
use std::boxed::Box;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallInputs {
    pub input: Bytes,
    pub return_memory_offset: Range<usize>,
    pub gas_limit: u64,
    pub bytecode_address: Address,
    pub target_address: Address,
    pub caller: Address,
    pub value: CallValue,
    pub scheme: CallScheme,
    pub is_static: bool,
    pub is_eof: bool,
}

impl CallInputs {
    pub fn new(tx_env: &TxEnv, gas_limit: u64) -> Option<Self> {
        match tx_env.transact_to {
            TransactTo::Call(target_address) => Some(Self {
                input: tx_env.data.clone(),
                gas_limit,
                target_address,
                bytecode_address: target_address,
                caller: tx_env.caller,
                value: CallValue::Transfer(tx_env.value),
                scheme: CallScheme::Call,
                is_static: false,
                is_eof: false,
                return_memory_offset: 0..0,
            }),
            _ => None,
        }
    }

    pub fn new_boxed(tx_env: &TxEnv, gas_limit: u64) -> Option<Box<Self>> {
        Self::new(tx_env, gas_limit).map(Box::new)
    }

    #[inline]
    pub fn transfers_value(&self) -> bool {
        self.value.transfer().is_some_and(|x| x > U256::ZERO)
    }

    #[inline]
    pub const fn transfer_value(&self) -> Option<U256> {
        self.value.transfer()
    }

    #[inline]
    pub const fn apparent_value(&self) -> Option<U256> {
        self.value.apparent()
    }

    #[inline]
    pub const fn transfer_from(&self) -> Address {
        self.caller
    }

    #[inline]
    pub const fn transfer_to(&self) -> Address {
        self.target_address
    }

    #[inline]
    pub const fn call_value(&self) -> U256 {
        self.value.get()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CallScheme {
    Call,
    CallCode,
    DelegateCall,
    StaticCall,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CallValue {
    Transfer(U256),
    Apparent(U256),
}

impl Default for CallValue {
    #[inline]
    fn default() -> Self {
        CallValue::Transfer(U256::ZERO)
    }
}

impl CallValue {
    #[inline]
    pub const fn get(&self) -> U256 {
        match *self {
            Self::Transfer(value) | Self::Apparent(value) => value,
        }
    }

    #[inline]
    pub const fn transfer(&self) -> Option<U256> {
        match *self {
            Self::Transfer(transfer) => Some(transfer),
            Self::Apparent(_) => None,
        }
    }

    #[inline]
    pub const fn is_transfer(&self) -> bool {
        matches!(self, Self::Transfer(_))
    }

    #[inline]
    pub const fn apparent(&self) -> Option<U256> {
        match *self {
            Self::Transfer(_) => None,
            Self::Apparent(apparent) => Some(apparent),
        }
    }

    #[inline]
    pub const fn is_apparent(&self) -> bool {
        matches!(self, Self::Apparent(_))
    }
}
