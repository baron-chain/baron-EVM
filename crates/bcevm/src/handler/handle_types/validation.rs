use crate::{
    handler::mainnet,
    primitives::{db::Database, EVMError, Env, Spec},
    Context,
};
use std::sync::Arc;

pub type ValidateEnvHandle<'a, DB> =
    Arc<dyn Fn(&Env) -> Result<(), EVMError<<DB as Database>::Error>> + 'a>;

pub type ValidateTxEnvAgainstState<'a, EXT, DB> =
    Arc<dyn Fn(&mut Context<EXT, DB>) -> Result<(), EVMError<<DB as Database>::Error>> + 'a>;

pub type ValidateInitialTxGasHandle<'a, DB> =
    Arc<dyn Fn(&Env) -> Result<u64, EVMError<<DB as Database>::Error>> + 'a>;

pub struct ValidationHandler<'a, EXT, DB: Database> {
    pub initial_tx_gas: ValidateInitialTxGasHandle<'a, DB>,
    pub tx_against_state: ValidateTxEnvAgainstState<'a, EXT, DB>,
    pub env: ValidateEnvHandle<'a, DB>,
}

impl<'a, EXT: 'a, DB: Database + 'a> ValidationHandler<'a, EXT, DB> {
    pub fn new<SPEC: Spec + 'a>() -> Self {
        Self {
            initial_tx_gas: Arc::new(mainnet::validate_initial_tx_gas::<SPEC, DB>),
            env: Arc::new(mainnet::validate_env::<SPEC, DB>),
            tx_against_state: Arc::new(mainnet::validate_tx_against_state::<SPEC, EXT, DB>),
        }
    }
}

impl<'a, EXT, DB: Database> ValidationHandler<'a, EXT, DB> {
    pub fn env(&self, env: &Env) -> Result<(), EVMError<DB::Error>> {
        (self.env)(env)
    }

    pub fn initial_tx_gas(&self, env: &Env) -> Result<u64, EVMError<DB::Error>> {
        (self.initial_tx_gas)(env)
    }

    pub fn tx_against_state(
        &self,
        context: &mut Context<EXT, DB>,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.tx_against_state)(context)
    }
}
