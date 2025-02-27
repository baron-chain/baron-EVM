use crate::{
    precompile::{PrecompileSpecId, Precompiles},
    primitives::{
        db::Database,
        Account, EVMError, Env, Spec,
        SpecId::{CANCUN, SHANGHAI},
        TransactTo, U256,
    },
    Context, ContextPrecompiles,
};

#[inline]
pub fn load_precompiles<SPEC: Spec, DB: Database>() -> ContextPrecompiles<DB> {
    Precompiles::new(PrecompileSpecId::from_spec_id(SPEC::SPEC_ID))
        .clone()
        .into()
}

#[inline]
pub fn load_accounts<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    context.evm.journaled_state.set_spec_id(SPEC::SPEC_ID);
    if SPEC::enabled(SHANGHAI) {
        context.evm.inner.journaled_state.initial_account_load(
            context.evm.inner.env.block.coinbase,
            &[],
            &mut context.evm.inner.db,
        )?;
    }
    context.evm.load_access_list()?;
    Ok(())
}

#[inline]
pub fn deduct_caller_inner<SPEC: Spec>(caller_account: &mut Account, env: &Env) {
    let mut gas_cost = U256::from(env.tx.gas_limit).saturating_mul(env.effective_gas_price());
    if SPEC::enabled(CANCUN) {
        let data_fee = env.calc_data_fee().expect("already checked");
        gas_cost = gas_cost.saturating_add(data_fee);
    }
    caller_account.info.balance = caller_account.info.balance.saturating_sub(gas_cost);
    if matches!(env.tx.transact_to, TransactTo::Call(_)) {
        caller_account.info.nonce = caller_account.info.nonce.saturating_add(1);
    }
    caller_account.mark_touch();
}

#[inline]
pub fn deduct_caller<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    let (caller_account, _) = context
        .evm
        .inner
        .journaled_state
        .load_account(context.evm.inner.env.tx.caller, &mut context.evm.inner.db)?;
    deduct_caller_inner::<SPEC>(caller_account, &context.evm.inner.env);
    Ok(())
}
