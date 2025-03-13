//BCMOD [ERR#0x0ac03e] [ERR#0x0ac03e] [ERR#0x0ac03e] [ERR#0x0ac03e]
mod context_precompiles;
mod evm_context;
mod inner_evm_context;

pub use context_precompiles::{
    ContextPrecompile, ContextPrecompiles, ContextStatefulPrecompile, ContextStatefulPrecompileArc,
    ContextStatefulPrecompileBox, ContextStatefulPrecompileMut,
};
pub use evm_context::EvmContext;
pub use inner_evm_context::InnebcevmContext;

use crate::{
    db::{Database, EmptyDB},
    primitives::HandlerCfg,
};
use std::boxed::Box;

pub struct Context<EXT, DB: Database> {
    pub evm: EvmContext<DB>,
    pub external: EXT,
}

impl<EXT: Clone, DB: Database + Clone> Clone for Context<EXT, DB>
where
    DB::Error: Clone,
{
    fn clone(&self) -> Self {
        Self {
            evm: self.evm.clone(),
            external: self.external.clone(),
        }
    }
}

impl Default for Context<(), EmptyDB> {
    fn default() -> Self {
        Self::new_empty()
    }
}

impl Context<(), EmptyDB> {
    pub fn new_empty() -> Self {
        Self {
            evm: EvmContext::new(EmptyDB::new()),
            external: (),
        }
    }
}

impl<DB: Database> Context<(), DB> {
    pub fn new_with_db(db: DB) -> Self {
        Self {
            evm: EvmContext::new_with_env(db, Box::default()),
            external: (),
        }
    }
}

impl<EXT, DB: Database> Context<EXT, DB> {
    pub fn new(evm: EvmContext<DB>, external: EXT) -> Self {
        Self { evm, external }
    }
}

pub struct ContextWithHandlerCfg<EXT, DB: Database> {
    pub context: Context<EXT, DB>,
    pub cfg: HandlerCfg,
}

impl<EXT, DB: Database> ContextWithHandlerCfg<EXT, DB> {
    pub fn new(context: Context<EXT, DB>, cfg: HandlerCfg) -> Self {
        Self { context, cfg }
    }
}

impl<EXT: Clone, DB: Database + Clone> Clone for ContextWithHandlerCfg<EXT, DB>
where
    DB::Error: Clone,
{
    fn clone(&self) -> Self {
        Self {
            context: self.context.clone(),
            cfg: self.cfg,
        }
    }
}
