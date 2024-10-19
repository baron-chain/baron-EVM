use crate::{
    precompile::{Precompile, PrecompileResult},
    primitives::{db::Database, Address, Bytes, HashMap},
};
use core::ops::{Deref, DerefMut};
use dyn_clone::DynClone;
use bcevm_precompile::Precompiles;
use std::{sync::Arc, boxed::Box};

use super::InnebcevmContext;

pub enum ContextPrecompile<DB: Database> {
    Ordinary(Precompile),
    ContextStateful(Arc<dyn ContextStatefulPrecompile<DB>>),
    ContextStatefulMut(Box<dyn ContextStatefulPrecompileMut<DB>>),
}

impl<DB: Database> Clone for ContextPrecompile<DB> {
    fn clone(&self) -> Self {
        match self {
            Self::Ordinary(p) => Self::Ordinary(p.clone()),
            Self::ContextStateful(p) => Self::ContextStateful(p.clone()),
            Self::ContextStatefulMut(p) => Self::ContextStatefulMut(p.clone()),
        }
    }
}

#[derive(Clone, Default)]
pub struct ContextPrecompiles<DB: Database> {
    inner: HashMap<Address, ContextPrecompile<DB>>,
}

impl<DB: Database> ContextPrecompiles<DB> {
    #[inline]
    pub fn addresses(&self) -> impl Iterator<Item = &Address> {
        self.inner.keys()
    }

    #[inline]
    pub fn extend(&mut self, other: impl IntoIterator<Item = impl Into<(Address, ContextPrecompile<DB>)>>) {
        self.inner.extend(other.into_iter().map(Into::into));
    }

    #[inline]
    pub fn call(&mut self, address: Address, bytes: &Bytes, gas_price: u64, evmctx: &mut InnebcevmContext<DB>) -> Option<PrecompileResult> {
        self.inner.get_mut(&address).map(|precompile| match precompile {
            ContextPrecompile::Ordinary(p) => p.call(bytes, gas_price, &evmctx.env),
            ContextPrecompile::ContextStatefulMut(p) => p.call_mut(bytes, gas_price, evmctx),
            ContextPrecompile::ContextStateful(p) => p.call(bytes, gas_price, evmctx),
        })
    }
}

impl<DB: Database> Deref for ContextPrecompiles<DB> {
    type Target = HashMap<Address, ContextPrecompile<DB>>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<DB: Database> DerefMut for ContextPrecompiles<DB> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

pub trait ContextStatefulPrecompile<DB: Database>: Sync + Send {
    fn call(&self, bytes: &Bytes, gas_price: u64, evmctx: &mut InnebcevmContext<DB>) -> PrecompileResult;
}

pub trait ContextStatefulPrecompileMut<DB: Database>: DynClone + Send + Sync {
    fn call_mut(&mut self, bytes: &Bytes, gas_price: u64, evmctx: &mut InnebcevmContext<DB>) -> PrecompileResult;
}

dyn_clone::clone_trait_object!(<DB> ContextStatefulPrecompileMut<DB>);

impl<DB: Database> From<Precompile> for ContextPrecompile<DB> {
    fn from(p: Precompile) -> Self { ContextPrecompile::Ordinary(p) }
}

impl<DB: Database> From<Precompiles> for ContextPrecompiles<DB> {
    fn from(p: Precompiles) -> Self {
        ContextPrecompiles { inner: p.inner.into_iter().map(|(k, v)| (k, v.into())).collect() }
    }
}

impl<DB: Database> From<&Precompiles> for ContextPrecompiles<DB> {
    fn from(p: &Precompiles) -> Self {
        ContextPrecompiles { inner: p.inner.iter().map(|(&k, v)| (k, v.clone().into())).collect() }
    }
}
