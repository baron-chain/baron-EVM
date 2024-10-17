use crate::{
    frame::EOFCreateFrame,
    handler::mainnet,
    interpreter::{CallInputs, CreateInputs, SharedMemory},
    primitives::{db::Database, EVMError, Spec},
    CallFrame, Context, CreateFrame, Frame, FrameOrResult, FrameResult,
};
use std::{boxed::Box, sync::Arc};

use bcevm_interpreter::{
    CallOutcome, CreateOutcome, EOFCreateInput, EOFCreateOutcome, InterpreterResult,
};

pub type LastFrameReturnHandle<'a, EXT, DB> = Arc<
    dyn Fn(&mut Context<EXT, DB>, &mut FrameResult) -> Result<(), EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameCallHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<CallInputs>,
        ) -> Result<FrameOrResult, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameCallReturnHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<CallFrame>,
            InterpreterResult,
        ) -> Result<CallOutcome, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type InsertCallOutcomeHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            &mut Frame,
            &mut SharedMemory,
            CallOutcome,
        ) -> Result<(), EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameCreateHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<CreateInputs>,
        ) -> Result<FrameOrResult, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameCreateReturnHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<CreateFrame>,
            InterpreterResult,
        ) -> Result<CreateOutcome, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type InsertCreateOutcomeHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            &mut Frame,
            CreateOutcome,
        ) -> Result<(), EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameEOFCreateHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<EOFCreateInput>,
        ) -> Result<FrameOrResult, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type FrameEOFCreateReturnHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Box<EOFCreateFrame>,
            InterpreterResult,
        ) -> Result<EOFCreateOutcome, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type InsertEOFCreateOutcomeHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            &mut Frame,
            EOFCreateOutcome,
        ) -> Result<(), EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub struct ExecutionHandler<'a, EXT, DB: Database> {
    pub last_frame_return: LastFrameReturnHandle<'a, EXT, DB>,
    pub call: FrameCallHandle<'a, EXT, DB>,
    pub call_return: FrameCallReturnHandle<'a, EXT, DB>,
    pub insert_call_outcome: InsertCallOutcomeHandle<'a, EXT, DB>,
    pub create: FrameCreateHandle<'a, EXT, DB>,
    pub create_return: FrameCreateReturnHandle<'a, EXT, DB>,
    pub insert_create_outcome: InsertCreateOutcomeHandle<'a, EXT, DB>,
    pub eofcreate: FrameEOFCreateHandle<'a, EXT, DB>,
    pub eofcreate_return: FrameEOFCreateReturnHandle<'a, EXT, DB>,
    pub insert_eofcreate_outcome: InsertEOFCreateOutcomeHandle<'a, EXT, DB>,
}

impl<'a, EXT: 'a, DB: Database + 'a> ExecutionHandler<'a, EXT, DB> {
    pub fn new<SPEC: Spec + 'a>() -> Self {
        Self {
            last_frame_return: Arc::new(mainnet::last_frame_return::<SPEC, EXT, DB>),
            call: Arc::new(mainnet::call::<SPEC, EXT, DB>),
            call_return: Arc::new(mainnet::call_return::<EXT, DB>),
            insert_call_outcome: Arc::new(mainnet::insert_call_outcome),
            create: Arc::new(mainnet::create::<SPEC, EXT, DB>),
            create_return: Arc::new(mainnet::create_return::<SPEC, EXT, DB>),
            insert_create_outcome: Arc::new(mainnet::insert_create_outcome),
            eofcreate: Arc::new(mainnet::eofcreate::<SPEC, EXT, DB>),
            eofcreate_return: Arc::new(mainnet::eofcreate_return::<SPEC, EXT, DB>),
            insert_eofcreate_outcome: Arc::new(mainnet::insert_eofcreate_outcome),
        }
    }
}

impl<'a, EXT, DB: Database> ExecutionHandler<'a, EXT, DB> {
    #[inline]
    pub fn last_frame_return(
        &self,
        context: &mut Context<EXT, DB>,
        frame_result: &mut FrameResult,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.last_frame_return)(context, frame_result)
    }

    #[inline]
    pub fn call(
        &self,
        context: &mut Context<EXT, DB>,
        inputs: Box<CallInputs>,
    ) -> Result<FrameOrResult, EVMError<DB::Error>> {
        (self.call)(context, inputs.clone())
    }

    #[inline]
    pub fn call_return(
        &self,
        context: &mut Context<EXT, DB>,
        frame: Box<CallFrame>,
        interpreter_result: InterpreterResult,
    ) -> Result<CallOutcome, EVMError<DB::Error>> {
        (self.call_return)(context, frame, interpreter_result)
    }

    #[inline]
    pub fn insert_call_outcome(
        &self,
        context: &mut Context<EXT, DB>,
        frame: &mut Frame,
        shared_memory: &mut SharedMemory,
        outcome: CallOutcome,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.insert_call_outcome)(context, frame, shared_memory, outcome)
    }

    #[inline]
    pub fn create(
        &self,
        context: &mut Context<EXT, DB>,
        inputs: Box<CreateInputs>,
    ) -> Result<FrameOrResult, EVMError<DB::Error>> {
        (self.create)(context, inputs)
    }

    #[inline]
    pub fn create_return(
        &self,
        context: &mut Context<EXT, DB>,
        frame: Box<CreateFrame>,
        interpreter_result: InterpreterResult,
    ) -> Result<CreateOutcome, EVMError<DB::Error>> {
        (self.create_return)(context, frame, interpreter_result)
    }

    #[inline]
    pub fn insert_create_outcome(
        &self,
        context: &mut Context<EXT, DB>,
        frame: &mut Frame,
        outcome: CreateOutcome,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.insert_create_outcome)(context, frame, outcome)
    }

    #[inline]
    pub fn eofcreate(
        &self,
        context: &mut Context<EXT, DB>,
        inputs: Box<EOFCreateInput>,
    ) -> Result<FrameOrResult, EVMError<DB::Error>> {
        (self.eofcreate)(context, inputs)
    }

    #[inline]
    pub fn eofcreate_return(
        &self,
        context: &mut Context<EXT, DB>,
        frame: Box<EOFCreateFrame>,
        interpreter_result: InterpreterResult,
    ) -> Result<EOFCreateOutcome, EVMError<DB::Error>> {
        (self.eofcreate_return)(context, frame, interpreter_result)
    }

    #[inline]
    pub fn insert_eofcreate_outcome(
        &self,
        context: &mut Context<EXT, DB>,
        frame: &mut Frame,
        outcome: EOFCreateOutcome,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.insert_eofcreate_outcome)(context, frame, outcome)
    }
}
