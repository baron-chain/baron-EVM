use crate::{Bytes, Env};
use core::fmt;
use dyn_clone::DynClone;
use std::{boxed::Box, string::String, sync::Arc};

pub type PrecompileResult = Result<(u64, Bytes), PrecompileError>;
pub type StandardPrecompileFn = fn(&Bytes, u64) -> PrecompileResult;
pub type EnvPrecompileFn = fn(&Bytes, u64, env: &Env) -> PrecompileResult;

pub trait StatefulPrecompile: Sync + Send {
    fn call(&self, bytes: &Bytes, gas_price: u64, env: &Env) -> PrecompileResult;
}

pub trait StatefulPrecompileMut: DynClone + Send + Sync {
    fn call_mut(&mut self, bytes: &Bytes, gas_price: u64, env: &Env) -> PrecompileResult;
}

dyn_clone::clone_trait_object!(StatefulPrecompileMut);

pub type StatefulPrecompileArc = Arc<dyn StatefulPrecompile>;
pub type StatefulPrecompileBox = Box<dyn StatefulPrecompileMut>;

#[derive(Clone)]
pub enum Precompile {
    Standard(StandardPrecompileFn),
    Env(EnvPrecompileFn),
    Stateful(StatefulPrecompileArc),
    StatefulMut(StatefulPrecompileBox),
}

impl From<StandardPrecompileFn> for Precompile {
    fn from(p: StandardPrecompileFn) -> Self { Precompile::Standard(p) }
}

impl From<EnvPrecompileFn> for Precompile {
    fn from(p: EnvPrecompileFn) -> Self { Precompile::Env(p) }
}

impl From<StatefulPrecompileArc> for Precompile {
    fn from(p: StatefulPrecompileArc) -> Self { Precompile::Stateful(p) }
}

impl From<StatefulPrecompileBox> for Precompile {
    fn from(p: StatefulPrecompileBox) -> Self { Precompile::StatefulMut(p) }
}

impl fmt::Debug for Precompile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Precompile::Standard(_) => "Standard",
            Precompile::Env(_) => "Env",
            Precompile::Stateful(_) => "Stateful",
            Precompile::StatefulMut(_) => "StatefulMut",
        })
    }
}

impl Precompile {
    pub fn new_stateful<P: StatefulPrecompile + 'static>(p: P) -> Self {
        Self::Stateful(Arc::new(p))
    }

    pub fn new_stateful_mut<P: StatefulPrecompileMut + 'static>(p: P) -> Self {
        Self::StatefulMut(Box::new(p))
    }

    pub fn call(&mut self, bytes: &Bytes, gas_price: u64, env: &Env) -> PrecompileResult {
        match self {
            Precompile::Standard(p) => p(bytes, gas_price),
            Precompile::Env(p) => p(bytes, gas_price, env),
            Precompile::Stateful(p) => p.call(bytes, gas_price, env),
            Precompile::StatefulMut(p) => p.call_mut(bytes, gas_price, env),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrecompileError {
    OutOfGas,
    Blake2WrongLength,
    Blake2WrongFinalIndicatorFlag,
    ModexpExpOverflow,
    ModexpBaseOverflow,
    ModexpModOverflow,
    Bn128FieldPointNotAMember,
    Bn128AffineGFailedToCreate,
    Bn128PairLength,
    BlobInvalidInputLength,
    BlobMismatchedVersion,
    BlobVerifyKzgProofFailed,
    Other(String),
}

impl PrecompileError {
    pub fn other(err: impl Into<String>) -> Self { Self::Other(err.into()) }
}

#[cfg(feature = "std")]
impl std::error::Error for PrecompileError {}

impl fmt::Display for PrecompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::OutOfGas => "out of gas",
            Self::Blake2WrongLength => "wrong input length for blake2",
            Self::Blake2WrongFinalIndicatorFlag => "wrong final indicator flag for blake2",
            Self::ModexpExpOverflow => "modexp exp overflow",
            Self::ModexpBaseOverflow => "modexp base overflow",
            Self::ModexpModOverflow => "modexp mod overflow",
            Self::Bn128FieldPointNotAMember => "field point not a member of bn128 curve",
            Self::Bn128AffineGFailedToCreate => "failed to create affine g point for bn128 curve",
            Self::Bn128PairLength => "bn128 invalid pair length",
            Self::BlobInvalidInputLength => "invalid blob input length",
            Self::BlobMismatchedVersion => "mismatched blob version",
            Self::BlobVerifyKzgProofFailed => "verifying blob kzg proof failed",
            Self::Other(s) => s,
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn stateful_precompile_mut() {
        #[derive(Default, Clone)]
        struct MyPrecompile {}

        impl StatefulPrecompileMut for MyPrecompile {
            fn call_mut(&mut self, _bytes: &Bytes, _gas_price: u64, _env: &Env) -> PrecompileResult {
                PrecompileResult::Err(PrecompileError::OutOfGas)
            }
        }

        let mut p = Precompile::new_stateful_mut(MyPrecompile::default());
        if let Precompile::StatefulMut(p) = &mut p {
            let _ = p.call_mut(&Bytes::new(), 0, &Env::default());
        } else {
            panic!("not a state");
        }
    }
}
