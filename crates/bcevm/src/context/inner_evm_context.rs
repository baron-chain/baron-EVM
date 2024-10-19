use crate::{
    db::Database,
    interpreter::{
        analysis::to_analysed, gas, return_ok, Contract, CreateInputs, EOFCreateInput, Gas,
        InstructionResult, Interpreter, InterpreterResult, LoadAccountResult, SStoreResult,
        SelfDestructResult, MAX_CODE_SIZE,
    },
    journaled_state::JournaledState,
    primitives::{
        keccak256, Account, Address, AnalysisKind, Bytecode, Bytes, CreateScheme, EVMError, Env,
        Eof, HashSet, Spec, SpecId, B256, U256,
    },
    FrameOrResult, JournalCheckpoint, CALL_STACK_LIMIT,
};
use std::boxed::Box;

#[derive(Debug)]
pub struct InnebcevmContext<DB: Database> {
    pub env: Box<Env>,
    pub journaled_state: JournaledState,
    pub db: DB,
    pub error: Result<(), EVMError<DB::Error>>,
    #[cfg(feature = "optimism")]
    pub l1_block_info: Option<crate::optimism::L1BlockInfo>,
}

impl<DB: Database + Clone> Clone for InnebcevmContext<DB> where DB::Error: Clone {
    fn clone(&self) -> Self {
        Self {
            env: self.env.clone(),
            journaled_state: self.journaled_state.clone(),
            db: self.db.clone(),
            error: self.error.clone(),
            #[cfg(feature = "optimism")]
            l1_block_info: self.l1_block_info.clone(),
        }
    }
}

impl<DB: Database> InnebcevmContext<DB> {
    pub fn new(db: DB) -> Self {
        Self {
            env: Box::default(),
            journaled_state: JournaledState::new(SpecId::LATEST, HashSet::new()),
            db,
            error: Ok(()),
            #[cfg(feature = "optimism")]
            l1_block_info: None,
        }
    }

    pub fn new_with_env(db: DB, env: Box<Env>) -> Self {
        Self { env, ..Self::new(db) }
    }

    pub fn with_db<ODB: Database>(self, db: ODB) -> InnebcevmContext<ODB> {
        InnebcevmContext { env: self.env, journaled_state: self.journaled_state, db, error: Ok(()), #[cfg(feature = "optimism")] l1_block_info: self.l1_block_info }
    }

    pub const fn spec_id(&self) -> SpecId { self.journaled_state.spec }

    pub fn load_access_list(&mut self) -> Result<(), EVMError<DB::Error>> {
        for (address, slots) in self.env.tx.access_list.iter() {
            self.journaled_state.initial_account_load(*address, slots, &mut self.db)?;
        }
        Ok(())
    }

    pub fn env(&mut self) -> &mut Env { &mut self.env }

    pub fn take_error(&mut self) -> Result<(), EVMError<DB::Error>> {
        std::mem::replace(&mut self.error, Ok(()))
    }

    pub fn block_hash(&mut self, number: U256) -> Result<B256, EVMError<DB::Error>> {
        self.db.block_hash(number).map_err(EVMError::Database)
    }

    pub fn touch(&mut self, address: &Address) {
        self.journaled_state.touch(address);
    }

    pub fn load_account(&mut self, address: Address) -> Result<(&mut Account, bool), EVMError<DB::Error>> {
        self.journaled_state.load_account(address, &mut self.db)
    }

    pub fn load_account_exist(&mut self, address: Address) -> Result<LoadAccountResult, EVMError<DB::Error>> {
        self.journaled_state.load_account_exist(address, &mut self.db)
    }

    pub fn balance(&mut self, address: Address) -> Result<(U256, bool), EVMError<DB::Error>> {
        self.journaled_state.load_account(address, &mut self.db).map(|(acc, is_cold)| (acc.info.balance, is_cold))
    }

    pub fn code(&mut self, address: Address) -> Result<(Bytecode, bool), EVMError<DB::Error>> {
        self.journaled_state.load_code(address, &mut self.db).map(|(a, is_cold)| (a.info.code.clone().unwrap(), is_cold))
    }

    pub fn code_hash(&mut self, address: Address) -> Result<(B256, bool), EVMError<DB::Error>> {
        let (acc, is_cold) = self.journaled_state.load_code(address, &mut self.db)?;
        Ok((if acc.is_empty() { B256::ZERO } else { acc.info.code_hash }, is_cold))
    }

    pub fn sload(&mut self, address: Address, index: U256) -> Result<(U256, bool), EVMError<DB::Error>> {
        self.journaled_state.sload(address, index, &mut self.db)
    }

    pub fn sstore(&mut self, address: Address, index: U256, value: U256) -> Result<SStoreResult, EVMError<DB::Error>> {
        self.journaled_state.sstore(address, index, value, &mut self.db)
    }

    pub fn tload(&mut self, address: Address, index: U256) -> U256 {
        self.journaled_state.tload(address, index)
    }

    pub fn tstore(&mut self, address: Address, index: U256, value: U256) {
        self.journaled_state.tstore(address, index, value)
    }

    pub fn selfdestruct(&mut self, address: Address, target: Address) -> Result<SelfDestructResult, EVMError<DB::Error>> {
        self.journaled_state.selfdestruct(address, target, &mut self.db)
    }

    pub fn make_eofcreate_frame(&mut self, spec_id: SpecId, inputs: &EOFCreateInput) -> Result<FrameOrResult, EVMError<DB::Error>> {
        let return_error = |e| Ok(FrameOrResult::new_eofcreate_result(
            InterpreterResult { result: e, gas: Gas::new(inputs.gas_limit), output: Bytes::new() },
            inputs.created_address,
            inputs.return_memory_range.clone(),
        ));

        if self.journaled_state.depth() > CALL_STACK_LIMIT {
            return return_error(InstructionResult::CallTooDeep);
        }

        let (caller_balance, _) = self.balance(inputs.caller)?;
        if caller_balance < inputs.value {
            return return_error(InstructionResult::OutOfFunds);
        }

        if self.journaled_state.inc_nonce(inputs.caller).is_none() {
            return return_error(InstructionResult::Return);
        }

        self.journaled_state.load_account(inputs.created_address, &mut self.db)?;

        let checkpoint = match self.journaled_state.create_account_checkpoint(
            inputs.caller,
            inputs.created_address,
            inputs.value,
            spec_id,
        ) {
            Ok(checkpoint) => checkpoint,
            Err(e) => return return_error(e),
        };

        let contract = Contract::new(
            Bytes::new(),
            Bytecode::Eof(inputs.eof_init_code.clone()),
            None,
            inputs.created_address,
            inputs.caller,
            inputs.value,
        );

        let mut interpreter = Interpreter::new(contract, inputs.gas_limit, false);
        interpreter.set_is_eof_init();

        Ok(FrameOrResult::new_eofcreate_frame(
            inputs.created_address,
            inputs.return_memory_range.clone(),
            checkpoint,
            interpreter,
        ))
    }

    pub fn eofcreate_return<SPEC: Spec>(&mut self, interpreter_result: &mut InterpreterResult, address: Address, journal_checkpoint: JournalCheckpoint) {
        if interpreter_result.result != InstructionResult::ReturnContract {
            self.journaled_state.checkpoint_revert(journal_checkpoint);
            return;
        }

        self.journaled_state.checkpoint_commit();

        let bytecode = Eof::decode(interpreter_result.output.clone()).expect("Eof is already verified");
        self.journaled_state.set_code(address, Bytecode::Eof(bytecode));
    }

    pub fn make_create_frame(&mut self, spec_id: SpecId, inputs: &CreateInputs) -> Result<FrameOrResult, EVMError<DB::Error>> {
        let gas = Gas::new(inputs.gas_limit);
        let return_error = |e| Ok(FrameOrResult::new_create_result(
            InterpreterResult { result: e, gas, output: Bytes::new() },
            None,
        ));

        if self.journaled_state.depth() > CALL_STACK_LIMIT {
            return return_error(InstructionResult::CallTooDeep);
        }

        let (caller_balance, _) = self.balance(inputs.caller)?;
        if caller_balance < inputs.value {
            return return_error(InstructionResult::OutOfFunds);
        }

        let old_nonce = match self.journaled_state.inc_nonce(inputs.caller) {
            Some(nonce) => nonce - 1,
            None => return return_error(InstructionResult::Return),
        };

        let (created_address, init_code_hash) = match inputs.scheme {
            CreateScheme::Create => (inputs.caller.create(old_nonce), B256::ZERO),
            CreateScheme::Create2 { salt } => {
                let hash = keccak256(&inputs.init_code);
                (inputs.caller.create2(salt.to_be_bytes(), hash), hash)
            }
        };

        self.journaled_state.load_account(created_address, &mut self.db)?;

        let checkpoint = match self.journaled_state.create_account_checkpoint(
            inputs.caller,
            created_address,
            inputs.value,
            spec_id,
        ) {
            Ok(checkpoint) => checkpoint,
            Err(e) => return return_error(e),
        };

        let contract = Contract::new(
            Bytes::new(),
            Bytecode::new_raw(inputs.init_code.clone()),
            Some(init_code_hash),
            created_address,
            inputs.caller,
            inputs.value,
        );

        Ok(FrameOrResult::new_create_frame(
            created_address,
            checkpoint,
            Interpreter::new(contract, gas.limit(), false),
        ))
    }

    pub fn call_return(&mut self, interpreter_result: &InterpreterResult, journal_checkpoint: JournalCheckpoint) {
        if matches!(interpreter_result.result, return_ok!()) {
            self.journaled_state.checkpoint_commit();
        } else {
            self.journaled_state.checkpoint_revert(journal_checkpoint);
        }
    }

    pub fn create_return<SPEC: Spec>(&mut self, interpreter_result: &mut InterpreterResult, address: Address, journal_checkpoint: JournalCheckpoint) {
        if !matches!(interpreter_result.result, return_ok!()) {
            self.journaled_state.checkpoint_revert(journal_checkpoint);
            return;
        }

        if SPEC::enabled(SpecId::LONDON)
            && !interpreter_result.output.is_empty()
            && interpreter_result.output[0] == 0xEF
        {
            self.journaled_state.checkpoint_revert(journal_checkpoint);
            interpreter_result.result = InstructionResult::CreateContractStartingWithEF;
            return;
        }

        if SPEC::enabled(SpecId::SPURIOUS_DRAGON)
            && interpreter_result.output.len() > self.env.cfg.limit_contract_code_size.unwrap_or(MAX_CODE_SIZE)
        {
            self.journaled_state.checkpoint_revert(journal_checkpoint);
            interpreter_result.result = InstructionResult::CreateContractSizeLimit;
            return;
        }

        let gas_for_code = interpreter_result.output.len() as u64 * gas::CODEDEPOSIT;
        if !interpreter_result.gas.record_cost(gas_for_code) {
            if SPEC::enabled(SpecId::HOMESTEAD) {
                self.journaled_state.checkpoint_revert(journal_checkpoint);
                interpreter_result.result = InstructionResult::OutOfGas;
                return;
            } else {
                interpreter_result.output = Bytes::new();
            }
        }

        self.journaled_state.checkpoint_commit();

        let bytecode = match self.env.cfg.perf_analyse_created_bytecodes {
            AnalysisKind::Raw => Bytecode::new_raw(interpreter_result.output.clone()),
            AnalysisKind::Analyse => to_analysed(Bytecode::new_raw(interpreter_result.output.clone())),
        };

        self.journaled_state.set_code(address, bytecode);
        interpreter_result.result = InstructionResult::Return;
    }
}
