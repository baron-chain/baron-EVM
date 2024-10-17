use crate::{
    handler::mainnet,
    interpreter::Gas,
    primitives::{db::Database, EVMError, EVMResultGeneric, ResultAndState, Spec},
    Context, FrameResult,
};
use std::sync::Arc;

pub type ReimburseCallerHandle<'a, EXT, DB> =
    Arc<dyn Fn(&mut Context<EXT, DB>, &Gas) -> EVMResultGeneric<(), <DB as Database>::Error> + 'a>;

pub type RewardBeneficiaryHandle<'a, EXT, DB> = ReimburseCallerHandle<'a, EXT, DB>;

pub type OutputHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            FrameResult,
        ) -> Result<ResultAndState, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type EndHandle<'a, EXT, DB> = Arc<
    dyn Fn(
            &mut Context<EXT, DB>,
            Result<ResultAndState, EVMError<<DB as Database>::Error>>,
        ) -> Result<ResultAndState, EVMError<<DB as Database>::Error>>
        + 'a,
>;

pub type ClearHandle<'a, EXT, DB> = Arc<dyn Fn(&mut Context<EXT, DB>) + 'a>;

pub struct PostExecutionHandler<'a, EXT, DB: Database> {
    pub reimburse_caller: ReimburseCallerHandle<'a, EXT, DB>,
    pub reward_beneficiary: RewardBeneficiaryHandle<'a, EXT, DB>,
    pub output: OutputHandle<'a, EXT, DB>,
    pub end: EndHandle<'a, EXT, DB>,
    pub clear: ClearHandle<'a, EXT, DB>,
}

impl<'a, EXT: 'a, DB: Database + 'a> PostExecutionHandler<'a, EXT, DB> {
    pub fn new<SPEC: Spec + 'a>() -> Self {
        Self {
            reimburse_caller: Arc::new(mainnet::reimburse_caller::<SPEC, EXT, DB>),
            reward_beneficiary: Arc::new(mainnet::reward_beneficiary::<SPEC, EXT, DB>),
            output: Arc::new(mainnet::output::<EXT, DB>),
            end: Arc::new(mainnet::end::<EXT, DB>),
            clear: Arc::new(mainnet::clear::<EXT, DB>),
        }
    }
}

impl<'a, EXT, DB: Database> PostExecutionHandler<'a, EXT, DB> {
    pub fn reimburse_caller(
        &self,
        context: &mut Context<EXT, DB>,
        gas: &Gas,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.reimburse_caller)(context, gas)
    }

    pub fn reward_beneficiary(
        &self,
        context: &mut Context<EXT, DB>,
        gas: &Gas,
    ) -> Result<(), EVMError<DB::Error>> {
        (self.reward_beneficiary)(context, gas)
    }

    pub fn output(
        &self,
        context: &mut Context<EXT, DB>,
        result: FrameResult,
    ) -> Result<ResultAndState, EVMError<DB::Error>> {
        (self.output)(context, result)
    }

    pub fn end(
        &self,
        context: &mut Context<EXT, DB>,
        end_output: Result<ResultAndState, EVMError<DB::Error>>,
    ) -> Result<ResultAndState, EVMError<DB::Error>> {
        (self.end)(context, end_output)
    }

    pub fn clear(&self, context: &mut Context<EXT, DB>) {
        (self.clear)(context)
    }
}
