use crate::{
    db::Database,
    frame::EOFCreateFrame,
    interpreter::{
        return_ok, return_revert, CallInputs, CreateInputs, CreateOutcome, Gas, InstructionResult,
        SharedMemory,
    },
    primitives::{EVMError, Env, Spec, SpecId},
    CallFrame, Context, CreateFrame, Frame, FrameOrResult, FrameResult,
};
use bcevm_interpreter::{CallOutcome, EOFCreateInput, EOFCreateOutcome, InterpreterResult};
use std::boxed::Box;

#[inline]
pub fn frame_return_with_refund_flag<SPEC: Spec>(
    env: &Env,
    frame_result: &mut FrameResult,
    refund_enabled: bool,
) {
    let instruction_result = frame_result.interpreter_result().result;
    let gas = frame_result.gas_mut();
    let remaining = gas.remaining();
    let refunded = gas.refunded();

    *gas = Gas::new_spent(env.tx.gas_limit);

    match instruction_result {
        return_ok!() => {
            gas.erase_cost(remaining);
            gas.record_refund(refunded);
        }
        return_revert!() => {
            gas.erase_cost(remaining);
        }
        _ => {}
    }

    if refund_enabled {
        gas.set_final_refund(SPEC::SPEC_ID.is_enabled_in(SpecId::LONDON));
    }
}

#[inline]
pub fn last_frame_return<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame_result: &mut FrameResult,
) -> Result<(), EVMError<DB::Error>> {
    frame_return_with_refund_flag::<SPEC>(&context.evm.env, frame_result, true);
    Ok(())
}

#[inline]
pub fn call<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    inputs: Box<CallInputs>,
) -> Result<FrameOrResult, EVMError<DB::Error>> {
    context.evm.make_call_frame(&inputs)
}

#[inline]
pub fn call_return<EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: Box<CallFrame>,
    interpreter_result: InterpreterResult,
) -> Result<CallOutcome, EVMError<DB::Error>> {
    context
        .evm
        .call_return(&interpreter_result, frame.frame_data.checkpoint);
    Ok(CallOutcome::new(
        interpreter_result,
        frame.return_memory_range,
    ))
}

#[inline]
pub fn insert_call_outcome<EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: &mut Frame,
    shared_memory: &mut SharedMemory,
    outcome: CallOutcome,
) -> Result<(), EVMError<DB::Error>> {
    context.evm.take_error()?;
    frame
        .frame_data_mut()
        .interpreter
        .insert_call_outcome(shared_memory, outcome);
    Ok(())
}

#[inline]
pub fn create<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    inputs: Box<CreateInputs>,
) -> Result<FrameOrResult, EVMError<DB::Error>> {
    context.evm.make_create_frame(SPEC::SPEC_ID, &inputs)
}

#[inline]
pub fn create_return<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: Box<CreateFrame>,
    mut interpreter_result: InterpreterResult,
) -> Result<CreateOutcome, EVMError<DB::Error>> {
    context.evm.create_return::<SPEC>(
        &mut interpreter_result,
        frame.created_address,
        frame.frame_data.checkpoint,
    );
    Ok(CreateOutcome::new(
        interpreter_result,
        Some(frame.created_address),
    ))
}

#[inline]
pub fn insert_create_outcome<EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: &mut Frame,
    outcome: CreateOutcome,
) -> Result<(), EVMError<DB::Error>> {
    context.evm.take_error()?;
    frame
        .frame_data_mut()
        .interpreter
        .insert_create_outcome(outcome);
    Ok(())
}

#[inline]
pub fn eofcreate<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    inputs: Box<EOFCreateInput>,
) -> Result<FrameOrResult, EVMError<DB::Error>> {
    context.evm.make_eofcreate_frame(SPEC::SPEC_ID, &inputs)
}

#[inline]
pub fn eofcreate_return<SPEC: Spec, EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: Box<EOFCreateFrame>,
    mut interpreter_result: InterpreterResult,
) -> Result<EOFCreateOutcome, EVMError<DB::Error>> {
    context.evm.eofcreate_return::<SPEC>(
        &mut interpreter_result,
        frame.created_address,
        frame.frame_data.checkpoint,
    );
    Ok(EOFCreateOutcome::new(
        interpreter_result,
        frame.created_address,
        frame.return_memory_range,
    ))
}

#[inline]
pub fn insert_eofcreate_outcome<EXT, DB: Database>(
    context: &mut Context<EXT, DB>,
    frame: &mut Frame,
    outcome: EOFCreateOutcome,
) -> Result<(), EVMError<DB::Error>> {
    core::mem::replace(&mut context.evm.error, Ok(()))?;
    frame
        .frame_data_mut()
        .interpreter
        .insert_eofcreate_outcome(outcome);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcevm_interpreter::primitives::CancunSpec;
    use bcevm_precompile::Bytes;

    fn call_last_frame_return(instruction_result: InstructionResult, gas: Gas) -> Gas {
        let mut env = Env::default();
        env.tx.gas_limit = 100;

        let mut first_frame = FrameResult::Call(CallOutcome::new(
            InterpreterResult {
                result: instruction_result,
                output: Bytes::new(),
                gas,
            },
            0..0,
        ));
        frame_return_with_refund_flag::<CancunSpec>(&env, &mut first_frame, true);
        *first_frame.gas()
    }

    #[test]
    fn test_consume_gas() {
        let gas = call_last_frame_return(InstructionResult::Stop, Gas::new(90));
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spent(), 10);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_consume_gas_with_refund() {
        let mut return_gas = Gas::new(90);
        return_gas.record_refund(30);

        let gas = call_last_frame_return(InstructionResult::Stop, return_gas);
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spent(), 10);
        assert_eq!(gas.refunded(), 2);

        let gas = call_last_frame_return(InstructionResult::Revert, return_gas);
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spent(), 10);
        assert_eq!(gas.refunded(), 0);
    }

    #[test]
    fn test_revert_gas() {
        let gas = call_last_frame_return(InstructionResult::Revert, Gas::new(90));
        assert_eq!(gas.remaining(), 90);
        assert_eq!(gas.spent(), 10);
        assert_eq!(gas.refunded(), 0);
    }
}
