mod call_inputs;
mod call_outcome;
mod create_inputs;
mod create_outcome;
mod eof_create_inputs;
mod eof_create_outcome;

pub use call_inputs::{CallInputs, CallScheme, CallValue};
pub use call_outcome::CallOutcome;
pub use create_inputs::{CreateInputs, CreateScheme};
pub use create_outcome::CreateOutcome;
pub use eof_create_inputs::EOFCreateInput;
pub use eof_create_outcome::EOFCreateOutcome;

use crate::InterpreterResult;
use std::boxed::Box;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InterpreterAction {
    Call { inputs: Box<CallInputs> },
    Create { inputs: Box<CreateInputs> },
    EOFCreate { inputs: Box<EOFCreateInput> },
    Return { result: InterpreterResult },
    #[default]
    None,
}

impl InterpreterAction {
    #[inline]
    pub fn is_call(&self) -> bool {
        matches!(self, Self::Call { .. })
    }

    #[inline]
    pub fn is_create(&self) -> bool {
        matches!(self, Self::Create { .. })
    }

    #[inline]
    pub fn is_return(&self) -> bool {
        matches!(self, Self::Return { .. })
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    #[inline]
    pub fn is_some(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    pub fn into_result_return(self) -> Option<InterpreterResult> {
        if let Self::Return { result } = self {
            Some(result)
        } else {
            None
        }
    }
}
