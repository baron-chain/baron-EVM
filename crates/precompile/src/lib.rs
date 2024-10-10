#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
#[cfg(not(feature = "std"))]
extern crate alloc as std;

pub mod blake2;
pub mod bn128;
pub mod hash;
pub mod identity;
#[cfg(feature = "c-kzg")]
pub mod kzg_point_evaluation;
pub mod modexp;
pub mod secp256k1;
pub mod utilities;

use core::hash::Hash;
use once_cell::race::OnceBox;
pub use bcevm_primitives as primitives;
pub use bcevm_primitives::{precompile::PrecompileError as Error, precompile::*, Address, Bytes, HashMap, Log, B256};
use std::{boxed::Box, vec::Vec};

pub fn calc_linear_cost_u32(len: usize, base: u64, word: u64) -> u64 {
    (len as u64 + 32 - 1) / 32 * word + base
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct PrecompileOutput {
    pub cost: u64,
    pub output: Vec<u8>,
    pub logs: Vec<Log>,
}

impl PrecompileOutput {
    pub fn without_logs(cost: u64, output: Vec<u8>) -> Self {
        Self { cost, output, logs: Vec::new() }
    }
}

#[derive(Clone, Default, Debug)]
pub struct Precompiles {
    pub inner: HashMap<Address, Precompile>,
}

impl Precompiles {
    pub fn new(spec: PrecompileSpecId) -> &'static Self {
        match spec {
            PrecompileSpecId::HOMESTEAD => Self::homestead(),
            PrecompileSpecId::BYZANTIUM => Self::byzantium(),
            PrecompileSpecId::ISTANBUL => Self::istanbul(),
            PrecompileSpecId::BERLIN => Self::berlin(),
            PrecompileSpecId::CANCUN => Self::cancun(),
            PrecompileSpecId::LATEST => Self::latest(),
        }
    }

    pub fn homestead() -> &'static Self {
        static INSTANCE: OnceBox<Precompiles> = OnceBox::new();
        INSTANCE.get_or_init(|| {
            let mut precompiles = Precompiles::default();
            precompiles.extend([secp256k1::ECRECOVER, hash::SHA256, hash::RIPEMD160, identity::FUN]);
            Box::new(precompiles)
        })
    }

    pub fn byzantium() -> &'static Self {
        static INSTANCE: OnceBox<Precompiles> = OnceBox::new();
        INSTANCE.get_or_init(|| {
            let mut precompiles = Self::homestead().clone();
            precompiles.extend([bn128::add::BYZANTIUM, bn128::mul::BYZANTIUM, bn128::pair::BYZANTIUM, modexp::BYZANTIUM]);
            Box::new(precompiles)
        })
    }

    pub fn istanbul() -> &'static Self {
        static INSTANCE: OnceBox<Precompiles> = OnceBox::new();
        INSTANCE.get_or_init(|| {
            let mut precompiles = Self::byzantium().clone();
            precompiles.extend([blake2::FUN, bn128::add::ISTANBUL, bn128::mul::ISTANBUL, bn128::pair::ISTANBUL]);
            Box::new(precompiles)
        })
    }

    pub fn berlin() -> &'static Self {
        static INSTANCE: OnceBox<Precompiles> = OnceBox::new();
        INSTANCE.get_or_init(|| {
            let mut precompiles = Self::istanbul().clone();
            precompiles.extend([modexp::BERLIN]);
            Box::new(precompiles)
        })
    }

    pub fn cancun() -> &'static Self {
        static INSTANCE: OnceBox<Precompiles> = OnceBox::new();
        INSTANCE.get_or_init(|| {
            let precompiles = Self::berlin().clone();
            #[cfg(feature = "c-kzg")]
            let precompiles = {
                let mut precompiles = precompiles;
                precompiles.extend([kzg_point_evaluation::POINT_EVALUATION]);
                precompiles
            };
            Box::new(precompiles)
        })
    }

    pub fn latest() -> &'static Self {
        Self::cancun()
    }

    pub fn addresses(&self) -> impl Iterator<Item = &Address> {
        self.inner.keys()
    }

    pub fn into_addresses(self) -> impl Iterator<Item = Address> {
        self.inner.into_keys()
    }

    pub fn contains(&self, address: &Address) -> bool {
        self.inner.contains_key(address)
    }

    pub fn get(&self, address: &Address) -> Option<&Precompile> {
        self.inner.get(address)
    }

    pub fn get_mut(&mut self, address: &Address) -> Option<&mut Precompile> {
        self.inner.get_mut(address)
    }

    pub fn is_empty(&self) -> bool {
        self.inner.len() == 0
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = PrecompileWithAddress>) {
        self.inner.extend(other.into_iter().map(Into::into));
    }
}

#[derive(Clone, Debug)]
pub struct PrecompileWithAddress(pub Address, pub Precompile);

impl From<(Address, Precompile)> for PrecompileWithAddress {
    fn from(value: (Address, Precompile)) -> Self {
        PrecompileWithAddress(value.0, value.1)
    }
}

impl From<PrecompileWithAddress> for (Address, Precompile) {
    fn from(value: PrecompileWithAddress) -> Self {
        (value.0, value.1)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PrecompileSpecId {
    HOMESTEAD,
    BYZANTIUM,
    ISTANBUL,
    BERLIN,
    CANCUN,
    LATEST,
}

impl PrecompileSpecId {
    pub const fn from_spec_id(spec_id: bcevm_primitives::SpecId) -> Self {
        use bcevm_primitives::SpecId::*;
        match spec_id {
            FRONTIER | FRONTIER_THAWING | HOMESTEAD | DAO_FORK | TANGERINE | SPURIOUS_DRAGON => Self::HOMESTEAD,
            BYZANTIUM | CONSTANTINOPLE | PETERSBURG => Self::BYZANTIUM,
            ISTANBUL | MUIR_GLACIER => Self::ISTANBUL,
            BERLIN | LONDON | ARROW_GLACIER | GRAY_GLACIER | MERGE | SHANGHAI => Self::BERLIN,
            CANCUN | PRAGUE => Self::CANCUN,
            LATEST => Self::LATEST,
            #[cfg(feature = "optimism")]
            BEDROCK | REGOLITH | CANYON => Self::BERLIN,
            #[cfg(feature = "optimism")]
            ECOTONE => Self::CANCUN,
        }
    }
}

#[inline]
pub const fn u64_to_address(x: u64) -> Address {
    let x = x.to_be_bytes();
    Address::new([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, x[0], x[1], x[2], x[3], x[4], x[5], x[6], x[7]])
}
