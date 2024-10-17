use bcevm_interpreter::gas;
use crate::{
    primitives::{db::Database, EVMError, Env, InvalidTransaction, Spec},
    Context,
};

pub fn validate_env<SPEC: Spec, DB: Database>(env: &Env) -> Result<(), EVMError<DB::Error>> {
    env.validate_block_env::<SPEC>()?;
    env.validate_tx::<SPEC>()?;
    Ok(())
}

pub fn validate_tx_against_state<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    let tx_caller = context.evm.env.tx.caller;
    let (caller_account, _) = context
        .evm
        .inner
        .journaled_state
        .load_account(tx_caller, &mut context.evm.inner.db)?;
    context
        .evm
        .inner
        .env
        .validate_tx_against_state::<SPEC>(caller_account)
        .map_err(EVMError::Transaction)?;
    Ok(())
}

pub fn validate_initial_tx_gas<SPEC: Spec, DB: Database>(
    env: &Env,
) -> Result<u64, EVMError<DB::Error>> {
    let input = &env.tx.data;
    let is_create = env.tx.transact_to.is_create();
    let access_list = &env.tx.access_list;
    let initcodes = &env.tx.eof_initcodes;
    let initial_gas_spend =
        gas::validate_initial_tx_gas(SPEC::SPEC_ID, input, is_create, access_list, initcodes);

    if initial_gas_spend > env.tx.gas_limit {
        return Err(InvalidTransaction::CallGasCostMoreThanGasLimit.into());
    }
    Ok(initial_gas_spend)
}
