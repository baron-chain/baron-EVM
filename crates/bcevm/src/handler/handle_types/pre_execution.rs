use crate::{
    handler::mainnet,
    primitives::{db::Database, EVMError, EVMResultGeneric, Spec},
    Context, ContextPrecompiles,
};
use std::sync::Arc;

pub type LoadPrecompilesHandle<'a, DB> = Arc<dyn Fn() -> ContextPrecompiles<DB> + 'a>;

pub type LoadAccountsHandle<'a, EXT, DB> =
    Arc<dyn Fn(&mut Context<EXT, DB>) -> Result<(), EVMError<<DB as Database>::Error>> + 'a>;

pub type DeductCallerHandle<'a, EXT, DB> =
    Arc<dyn Fn(&mut Context<EXT, DB>) -> EVMResultGeneric<(), <DB as Database>::Error> + 'a>;

pub struct PreExecutionHandler<'a, EXT, DB: Database> {
    pub load_precompiles: LoadPrecompilesHandle<'a, DB>,
    pub load_accounts: LoadAccountsHandle<'a, EXT, DB>,
    pub deduct_caller: DeductCallerHandle<'a, EXT, DB>,
}

impl<'a, EXT: 'a, DB: Database + 'a> PreExecutionHandler<'a, EXT, DB> {
    pub fn new<SPEC: Spec + 'a>() -> Self {
        Self {
            load_precompiles: Arc::new(mainnet::load_precompiles::<SPEC, DB>),
            load_accounts: Arc::new(mainnet::load_accounts::<SPEC, EXT, DB>),
            deduct_caller: Arc::new(mainnet::deduct_caller::<SPEC, EXT, DB>),
        }
    }
}

impl<'a, EXT, DB: Database> PreExecutionHandler<'a, EXT, DB> {
    pub fn deduct_caller(&self, context: &mut Context<EXT, DB>) -> Result<(), EVMError<DB::Error>> {
        (self.deduct_caller)(context)
    }

    pub fn load_accounts(&self, context: &mut Context<EXT, DB>) -> Result<(), EVMError<DB::Error>> {
        (self.load_accounts)(context)
    }

    pub fn load_precompiles(&self) -> ContextPrecompiles<DB> {
        (self.load_precompiles)()
    }
}
