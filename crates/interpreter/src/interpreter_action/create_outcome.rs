use crate::{Gas, InstructionResult, InterpreterResult};
use bcevm_primitives::{Address, Bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateOutcome {
    pub result: InterpreterResult,
    pub address: Option<Address>,
}

impl CreateOutcome {
    pub fn new(result: InterpreterResult, address: Option<Address>) -> Self {
        Self { result, address }
    }

    pub fn instruction_result(&self) -> &InstructionResult {
        &self.result.result
    }

    pub fn output(&self) -> &Bytes {
        &self.result.output
    }

    pub fn gas(&self) -> &Gas {
        &self.result.gas
    }
}
