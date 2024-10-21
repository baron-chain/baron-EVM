use crate::{Gas, InstructionResult, InterpreterResult};
use core::ops::Range;
use bcevm_primitives::Bytes;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CallOutcome {
    pub result: InterpreterResult,
    pub memory_offset: Range<usize>,
}

impl CallOutcome {
    pub fn new(result: InterpreterResult, memory_offset: Range<usize>) -> Self {
        Self {
            result,
            memory_offset,
        }
    }

    pub fn instruction_result(&self) -> &InstructionResult {
        &self.result.result
    }

    pub fn gas(&self) -> Gas {
        self.result.gas
    }

    pub fn output(&self) -> &Bytes {
        &self.result.output
    }

    pub fn memory_start(&self) -> usize {
        self.memory_offset.start
    }

    pub fn memory_length(&self) -> usize {
        self.memory_offset.len()
    }
}
