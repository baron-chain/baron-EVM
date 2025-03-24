//bcmod [ERR#0x003e0]
use crate::{
    builder::{EvmBuilder, HandlerStage, SetGenericStage},
    db::{Database, DatabaseCommit, EmptyDB},
    handler::Handler,
    interpreter::{
        opcode::InstructionTables, Host, Interpreter, InterpreterAction, LoadAccountResult,
        SStoreResult, SelfDestructResult, SharedMemory,
    },
    primitives::{
        specification::SpecId, Address, BlockEnv, Bytecode, CfgEnv, EVMError, EVMResult, Env,
        EnvWithHandlerCfg, ExecutionResult, HandlerCfg, Log, ResultAndState, TransactTo, TxEnv,
        B256, U256,
    },
    Context, ContextWithHandlerCfg, Frame, FrameOrResult, FrameResult,
};
use core::fmt;
use bcevm_interpreter::{CallInputs, CreateInputs};
use std::vec::Vec;

pub const CALL_STACK_LIMIT: u64 = 1024;

pub struct Evm<'a, EXT, DB: Database> {
    pub context: Context<EXT, DB>,
    pub handler: Handler<'a, Self, EXT, DB>,
}

impl<EXT, DB> fmt::Debug for Evm<'_, EXT, DB>
where
    EXT: fmt::Debug,
    DB: Database + fmt::Debug,
    DB::Error: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Evm")
            .field("evm context", &self.context.evm)
            .finish_non_exhaustive()
    }
}

impl<EXT, DB: Database + DatabaseCommit> Evm<'_, EXT, DB> {
    pub fn transact_commit(&mut self) -> Result<ExecutionResult, EVMError<DB::Error>> {
        let ResultAndState { result, state } = self.transact()?;
        self.context.evm.db.commit(state);
        Ok(result)
    }
}

impl<'a> Evm<'a, (), EmptyDB> {
    pub fn builder() -> EvmBuilder<'a, SetGenericStage, (), EmptyDB> {
        EvmBuilder::default()
    }
}

impl<'a, EXT, DB: Database> Evm<'a, EXT, DB> {
    pub fn new(
        mut context: Context<EXT, DB>,
        handler: Handler<'a, Self, EXT, DB>,
    ) -> Self {
        context.evm.journaled_state.set_spec_id(handler.cfg.spec_id);
        Self { context, handler }
    }

    pub fn modify(self) -> EvmBuilder<'a, HandlerStage, EXT, DB> {
        EvmBuilder::new(self)
    }

    pub fn spec_id(&self) -> SpecId {
        self.handler.cfg.spec_id
    }

    pub fn preverify_transaction(&mut self) -> Result<(), EVMError<DB::Error>> {
        let output = self.preverify_transaction_inner().map(|_| ());
        self.clear();
        output
    }

    fn clear(&mut self) {
        self.handler.post_execution().clear(&mut self.context);
    }

    pub fn transact_preverified(&mut self) -> EVMResult<DB::Error> {
        let initial_gas_spend = self.handler.validation().initial_tx_gas(&self.context.evm.env)?;
        let output = self.transact_preverified_inner(initial_gas_spend);
        let output = self.handler.post_execution().end(&mut self.context, output);
        self.clear();
        output
    }

    fn preverify_transaction_inner(&mut self) -> Result<u64, EVMError<DB::Error>> {
        self.handler.validation().env(&self.context.evm.env)?;
        let initial_gas_spend = self.handler.validation().initial_tx_gas(&self.context.evm.env)?;
        self.handler.validation().tx_against_state(&mut self.context)?;
        Ok(initial_gas_spend)
    }

    pub fn transact(&mut self) -> EVMResult<DB::Error> {
        let initial_gas_spend = self.preverify_transaction_inner()?;
        let output = self.transact_preverified_inner(initial_gas_spend);
        let output = self.handler.post_execution().end(&mut self.context, output);
        self.clear();
        output
    }

    pub fn handler_cfg(&self) -> &HandlerCfg {
        &self.handler.cfg
    }

    pub fn cfg(&self) -> &CfgEnv {
        &self.env().cfg
    }

    pub fn cfg_mut(&mut self) -> &mut CfgEnv {
        &mut self.context.evm.env.cfg
    }

    pub fn tx(&self) -> &TxEnv {
        &self.context.evm.env.tx
    }

    pub fn tx_mut(&mut self) -> &mut TxEnv {
        &mut self.context.evm.env.tx
    }

    pub fn db(&self) -> &DB {
        &self.context.evm.db
    }

    pub fn db_mut(&mut self) -> &mut DB {
        &mut self.context.evm.db
    }

    pub fn block(&self) -> &BlockEnv {
        &self.context.evm.env.block
    }

    pub fn block_mut(&mut self) -> &mut BlockEnv {
        &mut self.context.evm.env.block
    }

    pub fn modify_spec_id(&mut self, spec_id: SpecId) {
        self.handler.modify_spec_id(spec_id);
    }

    pub fn into_context(self) -> Context<EXT, DB> {
        self.context
    }

    pub fn into_db_and_env_with_handler_cfg(self) -> (DB, EnvWithHandlerCfg) {
        (
            self.context.evm.inner.db,
            EnvWithHandlerCfg {
                env: self.context.evm.inner.env,
                handler_cfg: self.handler.cfg,
            },
        )
    }

    pub fn into_context_with_handler_cfg(self) -> ContextWithHandlerCfg<EXT, DB> {
        ContextWithHandlerCfg::new(self.context, self.handler.cfg)
    }

    pub fn start_the_loop(
        &mut self,
        first_frame: Frame,
    ) -> Result<FrameResult, EVMError<DB::Error>> {
        let table = self.handler.take_instruction_table().expect("Instruction table should be present");
        let frame_result = match &table {
            InstructionTables::Plain(table) => self.run_the_loop(table, first_frame),
            InstructionTables::Boxed(table) => self.run_the_loop(table, first_frame),
        };
        self.handler.set_instruction_table(table);
        frame_result
    }

    pub fn run_the_loop<FN>(
        &mut self,
        instruction_table: &[FN; 256],
        first_frame: Frame,
    ) -> Result<FrameResult, EVMError<DB::Error>>
    where
        FN: Fn(&mut Interpreter, &mut Self),
    {
        let mut call_stack = Vec::with_capacity(1025);
        call_stack.push(first_frame);

        #[cfg(feature = "memory_limit")]
        let mut shared_memory = SharedMemory::new_with_memory_limit(self.context.evm.env.cfg.memory_limit);
        #[cfg(not(feature = "memory_limit"))]
        let mut shared_memory = SharedMemory::new();

        shared_memory.new_context();

        while let Some(mut stack_frame) = call_stack.last_mut() {
            let interpreter = &mut stack_frame.frame_data_mut().interpreter;
            let next_action = interpreter.run(shared_memory, instruction_table, self);
            self.context.evm.take_error()?;
            shared_memory = interpreter.take_memory();

            let exec = &mut self.handler.execution;
            let frame_or_result = match next_action {
                InterpreterAction::Call { inputs } => exec.call(&mut self.context, inputs)?,
                InterpreterAction::Create { inputs } => exec.create(&mut self.context, inputs)?,
                InterpreterAction::EOFCreate { inputs } => exec.eofcreate(&mut self.context, inputs)?,
                InterpreterAction::Return { result } => {
                    shared_memory.free_context();
                    let returned_frame = call_stack.pop().unwrap();
                    let ctx = &mut self.context;
                    FrameOrResult::Result(match returned_frame {
                        Frame::Call(frame) => FrameResult::Call(exec.call_return(ctx, frame, result)?),
                        Frame::Create(frame) => FrameResult::Create(exec.create_return(ctx, frame, result)?),
                        Frame::EOFCreate(frame) => FrameResult::EOFCreate(exec.eofcreate_return(ctx, frame, result)?),
                    })
                }
                InterpreterAction::None => unreachable!("InterpreterAction::None is not expected"),
            };

            match frame_or_result {
                FrameOrResult::Frame(frame) => {
                    shared_memory.new_context();
                    call_stack.push(frame);
                }
                FrameOrResult::Result(result) => {
                    if call_stack.is_empty() {
                        return Ok(result);
                    }
                    let ctx = &mut self.context;
                    match result {
                        FrameResult::Call(outcome) => exec.insert_call_outcome(ctx, call_stack.last_mut().unwrap(), &mut shared_memory, outcome)?,
                        FrameResult::Create(outcome) => exec.insert_create_outcome(ctx, call_stack.last_mut().unwrap(), outcome)?,
                        FrameResult::EOFCreate(outcome) => exec.insert_eofcreate_outcome(ctx, call_stack.last_mut().unwrap(), outcome)?,
                    }
                }
            }
        }
        unreachable!("Call stack should never be empty")
    }

    fn transact_preverified_inner(&mut self, initial_gas_spend: u64) -> EVMResult<DB::Error> {
        let ctx = &mut self.context;
        let pre_exec = self.handler.pre_execution();

        pre_exec.load_accounts(ctx)?;
        let precompiles = pre_exec.load_precompiles();
        ctx.evm.set_precompiles(precompiles);
        pre_exec.deduct_caller(ctx)?;

        let gas_limit = ctx.evm.env.tx.gas_limit - initial_gas_spend;

        let exec = self.handler.execution();
        let first_frame_or_result = match ctx.evm.env.tx.transact_to {
            TransactTo::Call(_) => exec.call(ctx, CallInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap())?,
            TransactTo::Create => exec.create(ctx, CreateInputs::new_boxed(&ctx.evm.env.tx, gas_limit).unwrap())?,
        };

        let mut result = match first_frame_or_result {
            FrameOrResult::Frame(first_frame) => self.start_the_loop(first_frame)?,
            FrameOrResult::Result(result) => result,
        };

        self.handler.execution().last_frame_return(ctx, &mut result)?;

        let post_exec = self.handler.post_execution();
        post_exec.reimburse_caller(ctx, result.gas())?;
        post_exec.reward_beneficiary(ctx, result.gas())?;
        post_exec.output(ctx, result)
    }
}

impl<EXT, DB: Database> Host for Evm<'_, EXT, DB> {
    fn env(&self) -> &Env {
        &self.context.evm.env
    }

    fn env_mut(&mut self) -> &mut Env {
        &mut self.context.evm.env
    }

    fn block_hash(&mut self, number: U256) -> Option<B256> {
        self.context.evm.block_hash(number).ok()
    }

    fn load_account(&mut self, address: Address) -> Option<LoadAccountResult> {
        self.context.evm.load_account_exist(address).ok()
    }

    fn balance(&mut self, address: Address) -> Option<(U256, bool)> {
        self.context.evm.balance(address).ok()
    }

    fn code(&mut self, address: Address) -> Option<(Bytecode, bool)> {
        self.context.evm.code(address).ok()
    }

    fn code_hash(&mut self, address: Address) -> Option<(B256, bool)> {
        self.context.evm.code_hash(address).ok()
    }

    fn sload(&mut self, address: Address, index: U256) -> Option<(U256, bool)> {
        self.context.evm.sload(address, index).ok()
    }

    fn sstore(&mut self, address: Address, index: U256, value: U256) -> Option<SStoreResult> {
        self.context.evm.sstore(address, index, value).ok()
    }

    fn tload(&mut self, address: Address, index: U256) -> U256 {
        self.context.evm.tload(address, index)
    }

    fn tstore(&mut self, address: Address, index: U256, value: U256) {
        self.context.evm.tstore(address, index, value)
    }

    fn log(&mut self, log: Log) {
        self.context.evm.journaled_state.log(log);
    }

    fn selfdestruct(&mut self, address: Address, target: Address) -> Option<SelfDestructResult> {
        self.context.evm.inner.journaled_state.selfdestruct(address, target, &mut self.context.evm.inner.db).ok()
    }
}
