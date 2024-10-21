use core::ops::Range;
use crate::{Gas, InstructionResult, InterpreterResult};
use bcevm_primitives::{Address, Bytes};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct EOFCreateOutcome {
    pub result: InterpreterResult,
    pub address: Address,
    pub return_memory_range: Range<usize>,
}

impl EOFCreateOutcome {
    pub fn new(
        result: InterpreterResult,
        address: Address,
        return_memory_range: Range<usize>,
    ) -> Self {
        Self {
            result,
            address,
            return_memory_range,
        }
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

    pub fn return_range(&self) -> Range<usize> {
        self.return_memory_range.clone()
    }
}
