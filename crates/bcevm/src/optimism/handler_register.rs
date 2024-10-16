use crate::{
    handler::{
        mainnet::{self, deduct_caller_inner},
        register::EvmHandler,
    },
    interpreter::{return_ok, return_revert, Gas, InstructionResult},
    optimism,
    primitives::{
        db::Database, spec_to_generic, Account, EVMError, Env, ExecutionResult, HaltReason,
        HashMap, InvalidTransaction, ResultAndState, Spec, SpecId, SpecId::REGOLITH, U256,
    },
    Context, FrameResult,
};
use core::ops::Mul;
use std::sync::Arc;

pub fn optimism_handle_register<DB: Database, EXT>(handler: &mut EvmHandler<'_, EXT, DB>) {
    spec_to_generic!(handler.cfg.spec_id, {
        handler.validation.env = Arc::new(validate_env::<SPEC, DB>);
        handler.validation.tx_against_state = Arc::new(validate_tx_against_state::<SPEC, EXT, DB>);
        handler.pre_execution.load_accounts = Arc::new(load_accounts::<SPEC, EXT, DB>);
        handler.pre_execution.deduct_caller = Arc::new(deduct_caller::<SPEC, EXT, DB>);
        handler.execution.last_frame_return = Arc::new(last_frame_return::<SPEC, EXT, DB>);
        handler.post_execution.reward_beneficiary = Arc::new(reward_beneficiary::<SPEC, EXT, DB>);
        handler.post_execution.output = Arc::new(output::<SPEC, EXT, DB>);
        handler.post_execution.end = Arc::new(end::<SPEC, EXT, DB>);
    });
}

pub fn validate_env<SPEC: Spec, DB: Database>(env: &Env) -> Result<(), EVMError<DB::Error>> {
    if env.tx.optimism.source_hash.is_some() {
        return Ok(());
    }
    env.validate_block_env::<SPEC>()?;

    let tx = &env.tx.optimism;
    if tx.is_system_transaction.unwrap_or(false) && SPEC::enabled(SpecId::REGOLITH) {
        return Err(InvalidTransaction::DepositSystemTxPostRegolith.into());
    }

    env.validate_tx::<SPEC>()
}

pub fn validate_tx_against_state<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    if context.evm.inner.env.tx.optimism.source_hash.is_some() {
        return Ok(());
    }
    mainnet::validate_tx_against_state::<SPEC, EXT, DB>(context)
}

#[inline]
pub fn last_frame_return<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame_result: &mut FrameResult,
) -> Result<(), EVMError<DB::Error>> {
    let env = context.evm.inner.env();
    let is_deposit = env.tx.optimism.source_hash.is_some();
    let tx_system = env.tx.optimism.is_system_transaction;
    let tx_gas_limit = env.tx.gas_limit;
    let is_regolith = SPEC::enabled(REGOLITH);

    let instruction_result = frame_result.interpreter_result().result;
    let gas = frame_result.gas_mut();
    let remaining = gas.remaining();
    let refunded = gas.refunded();
    *gas = Gas::new_spent(tx_gas_limit);

    match instruction_result {
        return_ok!() => {
            if !is_deposit || is_regolith {
                gas.erase_cost(remaining);
                gas.record_refund(refunded);
            } else if is_deposit && tx_system.unwrap_or(false) {
                gas.erase_cost(tx_gas_limit);
            }
        }
        return_revert!() => {
            if !is_deposit || is_regolith {
                gas.erase_cost(remaining);
            }
        }
        _ => {}
    }
    
    let is_gas_refund_disabled = env.cfg.is_gas_refund_disabled() || (is_deposit && !is_regolith);
    if !is_gas_refund_disabled {
        gas.set_final_refund(SPEC::SPEC_ID.is_enabled_in(SpecId::LONDON));
    }
    Ok(())
}

#[inline]
pub fn load_accounts<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    if context.evm.inner.env.tx.optimism.source_hash.is_none() {
        let l1_block_info = crate::optimism::L1BlockInfo::try_fetch(&mut context.evm.inner.db, SPEC::SPEC_ID)?;
        context.evm.inner.l1_block_info = Some(l1_block_info);
    }

    mainnet::load_accounts::<SPEC, EXT, DB>(context)
}

#[inline]
pub fn deduct_caller<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
) -> Result<(), EVMError<DB::Error>> {
    let (caller_account, _) = context.evm.inner.journaled_state.load_account(context.evm.inner.env.tx.caller, &mut context.evm.inner.db)?;

    if let Some(mint) = context.evm.inner.env.tx.optimism.mint {
        caller_account.info.balance += U256::from(mint);
    }

    deduct_caller_inner::<SPEC>(caller_account, &context.evm.inner.env);

    if context.evm.inner.env.tx.optimism.source_hash.is_none() {
        let Some(enveloped_tx) = &context.evm.inner.env.tx.optimism.enveloped_tx else {
            return Err(EVMError::Custom("Failed to load enveloped transaction".into()));
        };

        let tx_l1_cost = context.evm.inner.l1_block_info.as_ref()
            .expect("L1BlockInfo should be loaded")
            .calculate_tx_l1_cost(enveloped_tx, SPEC::SPEC_ID);
        if tx_l1_cost > caller_account.info.balance {
            return Err(EVMError::Transaction(InvalidTransaction::LackOfFundForMaxFee {
                fee: tx_l1_cost.into(),
                balance: caller_account.info.balance.into(),
            }));
        }
        caller_account.info.balance = caller_account.info.balance.saturating_sub(tx_l1_cost);
    }
    Ok(())
}

#[inline]
pub fn reward_beneficiary<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    gas: &Gas,
) -> Result<(), EVMError<DB::Error>> {
    let is_deposit = context.evm.inner.env.tx.optimism.source_hash.is_some();

    if !is_deposit {
        mainnet::reward_beneficiary::<SPEC, EXT, DB>(context, gas)?;

        let Some(l1_block_info) = &context.evm.inner.l1_block_info else {
            return Err(EVMError::Custom("Failed to load L1 block information".into()));
        };

        let Some(enveloped_tx) = &context.evm.inner.env.tx.optimism.enveloped_tx else {
            return Err(EVMError::Custom("Failed to load enveloped transaction".into()));
        };

        let l1_cost = l1_block_info.calculate_tx_l1_cost(enveloped_tx, SPEC::SPEC_ID);

        let (l1_fee_vault_account, _) = context.evm.inner.journaled_state
            .load_account(optimism::L1_FEE_RECIPIENT, &mut context.evm.inner.db)?;
        l1_fee_vault_account.mark_touch();
        l1_fee_vault_account.info.balance += l1_cost;

        let (base_fee_vault_account, _) = context.evm.inner.journaled_state
            .load_account(optimism::BASE_FEE_RECIPIENT, &mut context.evm.inner.db)?;
        base_fee_vault_account.mark_touch();
        base_fee_vault_account.info.balance += context.evm.inner.env.block.basefee
            .mul(U256::from(gas.spent() - gas.refunded() as u64));
    }
    Ok(())
}

#[inline]
pub fn output<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame_result: FrameResult,
) -> Result<ResultAndState, EVMError<DB::Error>> {
    let result = mainnet::output::<EXT, DB>(context, frame_result)?;

    if result.result.is_halt() {
        let is_deposit = context.evm.inner.env.tx.optimism.source_hash.is_some();
        if is_deposit && SPEC::enabled(REGOLITH) {
            return Err(EVMError::Transaction(InvalidTransaction::HaltedDepositPostRegolith));
        }
    }
    Ok(result)
}

#[inline]
pub fn end<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    evm_output: Result<ResultAndState, EVMError<DB::Error>>,
) -> Result<ResultAndState, EVMError<DB::Error>> {
    evm_output.or_else(|err| {
        if matches!(err, EVMError::Transaction(_))
            && context.evm.inner.env().tx.optimism.source_hash.is_some()
        {
            let caller = context.evm.inner.env().tx.caller;
            let account = {
                let mut acc = Account::from(context.evm.db.basic(caller).unwrap_or_default().unwrap_or_default());
                acc.info.nonce = acc.info.nonce.saturating_add(1);
                acc.info.balance = acc.info.balance.saturating_add(U256::from(
                    context.evm.inner.env().tx.optimism.mint.unwrap_or(0),
                ));
                acc.mark_touch();
                acc
            };
            let state = HashMap::from([(caller, account)]);

            let is_system_tx = context.evm.env().tx.optimism.is_system_transaction.unwrap_or(false);
            let gas_used = if SPEC::enabled(REGOLITH) || !is_system_tx {
                context.evm.inner.env().tx.gas_limit
            } else {
                0
            };

            Ok(ResultAndState {
                result: ExecutionResult::Halt {
                    reason: HaltReason::FailedDeposit,
                    gas_used,
                },
                state,
            })
        } else {
            Err(err)
        }
    })
}

#[cfg(test)]
mod tests {
    // Test module implementation remains unchanged
}
