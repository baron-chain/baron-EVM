use bcevm_interpreter::CallValue;
use super::inner_evm_context::InnebcevmContext;
use crate::{
    db::Database,
    interpreter::{return_ok, CallInputs, Contract, Gas, InstructionResult, Interpreter, InterpreterResult},
    primitives::{Address, Bytes, EVMError, Env, HashSet, U256},
    ContextPrecompiles, FrameOrResult, CALL_STACK_LIMIT,
};
use core::{fmt, ops::{Deref, DerefMut}};
use std::boxed::Box;

pub struct EvmContext<DB: Database> {
    pub inner: InnebcevmContext<DB>,
    pub precompiles: ContextPrecompiles<DB>,
}

impl<DB: Database + Clone> Clone for EvmContext<DB> where DB::Error: Clone {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            precompiles: ContextPrecompiles::default(),
        }
    }
}

impl<DB: Database + fmt::Debug> fmt::Debug for EvmContext<DB> where DB::Error: fmt::Debug {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EvmContext")
            .field("inner", &self.inner)
            .field("precompiles", &self.inner)
            .finish_non_exhaustive()
    }
}

impl<DB: Database> Deref for EvmContext<DB> {
    type Target = InnebcevmContext<DB>;
    fn deref(&self) -> &Self::Target { &self.inner }
}

impl<DB: Database> DerefMut for EvmContext<DB> {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.inner }
}

impl<DB: Database> EvmContext<DB> {
    pub fn new(db: DB) -> Self {
        Self {
            inner: InnebcevmContext::new(db),
            precompiles: ContextPrecompiles::default(),
        }
    }

    pub fn new_with_env(db: DB, env: Box<Env>) -> Self {
        Self {
            inner: InnebcevmContext::new_with_env(db, env),
            precompiles: ContextPrecompiles::default(),
        }
    }

    pub fn with_db<ODB: Database>(self, db: ODB) -> EvmContext<ODB> {
        EvmContext {
            inner: self.inner.with_db(db),
            precompiles: ContextPrecompiles::default(),
        }
    }

    pub fn set_precompiles(&mut self, precompiles: ContextPrecompiles<DB>) {
        self.journaled_state.warm_preloaded_addresses = precompiles.addresses().copied().collect();
        self.precompiles = precompiles;
    }

    fn call_precompile(&mut self, address: Address, input_data: &Bytes, gas: Gas) -> Option<InterpreterResult> {
        self.precompiles.call(address, input_data, gas.limit(), &mut self.inner).map(|out| {
            let mut result = InterpreterResult { result: InstructionResult::Return, gas, output: Bytes::new() };
            match out {
                Ok((gas_used, data)) => {
                    if result.gas.record_cost(gas_used) {
                        result.output = data;
                    } else {
                        result.result = InstructionResult::PrecompileOOG;
                    }
                }
                Err(e) => {
                    result.result = if e == crate::precompile::Error::OutOfGas {
                        InstructionResult::PrecompileOOG
                    } else {
                        InstructionResult::PrecompileError
                    };
                }
            }
            result
        })
    }

    pub fn make_call_frame(&mut self, inputs: &CallInputs) -> Result<FrameOrResult, EVMError<DB::Error>> {
        let gas = Gas::new(inputs.gas_limit);

        if self.journaled_state.depth() > CALL_STACK_LIMIT {
            return Ok(FrameOrResult::new_call_result(
                InterpreterResult { result: InstructionResult::CallTooDeep, gas, output: Bytes::new() },
                inputs.return_memory_offset.clone(),
            ));
        }

        let (account, _) = self.inner.journaled_state.load_code(inputs.bytecode_address, &mut self.inner.db)?;
        let code_hash = account.info.code_hash();
        let bytecode = account.info.code.clone().unwrap_or_default();
        let checkpoint = self.journaled_state.checkpoint();

        match inputs.value {
            CallValue::Transfer(value) if value == U256::ZERO => {
                self.load_account(inputs.target_address)?;
                self.journaled_state.touch(&inputs.target_address);
            }
            CallValue::Transfer(value) => {
                if let Some(result) = self.inner.journaled_state.transfer(&inputs.caller, &inputs.target_address, value, &mut self.inner.db)? {
                    self.journaled_state.checkpoint_revert(checkpoint);
                    return Ok(FrameOrResult::new_call_result(
                        InterpreterResult { result, gas, output: Bytes::new() },
                        inputs.return_memory_offset.clone(),
                    ));
                }
            }
            _ => {}
        };

        if let Some(result) = self.call_precompile(inputs.bytecode_address, &inputs.input, gas) {
            if matches!(result.result, return_ok!()) {
                self.journaled_state.checkpoint_commit();
            } else {
                self.journaled_state.checkpoint_revert(checkpoint);
            }
            Ok(FrameOrResult::new_call_result(result, inputs.return_memory_offset.clone()))
        } else if !bytecode.is_empty() {
            let contract = Contract::new_with_context(inputs.input.clone(), bytecode, Some(code_hash), inputs);
            Ok(FrameOrResult::new_call_frame(
                inputs.return_memory_offset.clone(),
                checkpoint,
                Interpreter::new(contract, gas.limit(), inputs.is_static),
            ))
        } else {
            self.journaled_state.checkpoint_commit();
            Ok(FrameOrResult::new_call_result(
                InterpreterResult { result: InstructionResult::Stop, gas, output: Bytes::new() },
                inputs.return_memory_offset.clone(),
            ))
        }
    }
}

#[cfg(any(test, feature = "test-utils"))]
pub(crate) mod test_utils {
    use super::*;
    use crate::{
        db::{CacheDB, EmptyDB},
        journaled_state::JournaledState,
        primitives::{address, SpecId, B256},
    };

    pub const MOCK_CALLER: Address = address!("0000000000000000000000000000000000000000");

    pub fn create_mock_call_inputs(to: Address) -> CallInputs {
        CallInputs {
            input: Bytes::new(),
            gas_limit: 0,
            bytecode_address: to,
            target_address: to,
            caller: MOCK_CALLER,
            value: CallValue::Transfer(U256::ZERO),
            scheme: bcevm_interpreter::CallScheme::Call,
            is_eof: false,
            is_static: false,
            return_memory_offset: 0..0,
        }
    }

    pub fn create_cache_db_evm_context_with_balance(
        env: Box<Env>,
        mut db: CacheDB<EmptyDB>,
        balance: U256,
    ) -> EvmContext<CacheDB<EmptyDB>> {
        db.insert_account_info(
            MOCK_CALLER,
            crate::primitives::AccountInfo {
                nonce: 0,
                balance,
                code_hash: B256::default(),
                code: None,
            },
        );
        create_cache_db_evm_context(env, db)
    }

    pub fn create_cache_db_evm_context(
        env: Box<Env>,
        db: CacheDB<EmptyDB>,
    ) -> EvmContext<CacheDB<EmptyDB>> {
        EvmContext {
            inner: InnebcevmContext {
                env,
                journaled_state: JournaledState::new(SpecId::CANCUN, HashSet::new()),
                db,
                error: Ok(()),
                #[cfg(feature = "optimism")]
                l1_block_info: None,
            },
            precompiles: ContextPrecompiles::default(),
        }
    }

    pub fn create_empty_evm_context(env: Box<Env>, db: EmptyDB) -> EvmContext<EmptyDB> {
        EvmContext {
            inner: InnebcevmContext {
                env,
                journaled_state: JournaledState::new(SpecId::CANCUN, HashSet::new()),
                db,
                error: Ok(()),
                #[cfg(feature = "optimism")]
                l1_block_info: None,
            },
            precompiles: ContextPrecompiles::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        db::{CacheDB, EmptyDB},
        primitives::{address, Bytecode},
        Frame, JournalEntry,
    };
    use test_utils::*;

    #[test]
    fn test_make_call_frame_stack_too_deep() {
        let mut context = create_empty_evm_context(Box::new(Env::default()), EmptyDB::default());
        context.journaled_state.depth = CALL_STACK_LIMIT as usize + 1;
        let contract = address!("dead10000000000000000000000000000001dead");
        let call_inputs = create_mock_call_inputs(contract);
        let res = context.make_call_frame(&call_inputs);
        let Ok(FrameOrResult::Result(err)) = res else { panic!("Expected FrameOrResult::Result") };
        assert_eq!(err.interpreter_result().result, InstructionResult::CallTooDeep);
    }

    #[test]
    fn test_make_call_frame_transfer_revert() {
        let mut evm_context = create_empty_evm_context(Box::new(Env::default()), EmptyDB::default());
        let contract = address!("dead10000000000000000000000000000001dead");
        let mut call_inputs = create_mock_call_inputs(contract);
        call_inputs.value = CallValue::Transfer(U256::from(1));
        let res = evm_context.make_call_frame(&call_inputs);
        let Ok(FrameOrResult::Result(result)) = res else { panic!("Expected FrameOrResult::Result") };
        assert_eq!(result.interpreter_result().result, InstructionResult::OutOfFunds);
        assert_eq!(evm_context.journaled_state.journal, vec![vec![JournalEntry::AccountLoaded { address: contract }]]);
        assert_eq!(evm_context.journaled_state.depth, 0);
    }

    #[test]
    fn test_make_call_frame_missing_code_context() {
        let mut context = create_cache_db_evm_context_with_balance(
            Box::new(Env::default()),
            CacheDB::new(EmptyDB::default()),
            U256::from(3_000_000_000_u128),
        );
        let contract = address!("dead10000000000000000000000000000001dead");
        let call_inputs = create_mock_call_inputs(contract);
        let res = context.make_call_frame(&call_inputs);
        let Ok(FrameOrResult::Result(result)) = res else { panic!("Expected FrameOrResult::Result") };
        assert_eq!(result.interpreter_result().result, InstructionResult::Stop);
    }

    #[test]
    fn test_make_call_frame_succeeds() {
        let mut cdb = CacheDB::new(EmptyDB::default());
        let bal = U256::from(3_000_000_000_u128);
        let by = Bytecode::new_raw(Bytes::from(vec![0x60, 0x00, 0x60, 0x00]));
        let contract = address!("dead10000000000000000000000000000001dead");
        cdb.insert_account_info(contract, crate::primitives::AccountInfo {
            nonce: 0,
            balance: bal,
            code_hash: by.clone().hash_slow(),
            code: Some(by),
        });
        let mut evm_context = create_cache_db_evm_context_with_balance(Box::new(Env::default()), cdb, bal);
        let call_inputs = create_mock_call_inputs(contract);
        let res = evm_context.make_call_frame(&call_inputs);
        let Ok(FrameOrResult::Frame(Frame::Call(call_frame))) = res else { panic!("Expected FrameOrResult::Frame(Frame::Call(..))") };
        assert_eq!(call_frame.return_memory_range, 0..0);
    }
}
