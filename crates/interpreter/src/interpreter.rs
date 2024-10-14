mod analysis;
mod contract;
#[cfg(feature = "serde")]
mod serde;
mod shared_memory;
mod stack;

pub use contract::Contract;
pub use shared_memory::{num_words, SharedMemory, EMPTY_SHARED_MEMORY};
pub use stack::{Stack, STACK_LIMIT};

use crate::{
    gas, primitives::Bytes, push, push_b256, return_ok, return_revert, CallOutcome, CreateOutcome,
    EOFCreateOutcome, FunctionStack, Gas, Host, InstructionResult, InterpreterAction,
};
use bcevm_primitives::{Address, Bytecode, Eof, U256};
use core::cmp::min;

#[derive(Debug)]
pub struct Interpreter {
    pub instruction_pointer: *const u8,
    pub gas: Gas,
    pub contract: Contract,
    pub instruction_result: InstructionResult,
    pub bytecode: Bytes,
    pub is_eof: bool,
    pub is_eof_init: bool,
    pub shared_memory: SharedMemory,
    pub stack: Stack,
    pub function_stack: FunctionStack,
    pub return_data_buffer: Bytes,
    pub is_static: bool,
    pub next_action: InterpreterAction,
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new(Contract::default(), 0, false)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct InterpreterResult {
    pub result: InstructionResult,
    pub output: Bytes,
    pub gas: Gas,
}

impl Interpreter {
    pub fn new(contract: Contract, gas_limit: u64, is_static: bool) -> Self {
        assert!(contract.bytecode.is_execution_ready(), "Contract is not execution ready");
        let is_eof = contract.bytecode.is_eof();
        let bytecode = contract.bytecode.bytecode().clone();
        Self {
            instruction_pointer: bytecode.as_ptr(),
            bytecode,
            contract,
            gas: Gas::new(gas_limit),
            instruction_result: InstructionResult::Continue,
            function_stack: FunctionStack::default(),
            is_static,
            is_eof,
            is_eof_init: false,
            return_data_buffer: Bytes::new(),
            shared_memory: EMPTY_SHARED_MEMORY,
            stack: Stack::new(),
            next_action: InterpreterAction::None,
        }
    }

    #[inline]
    pub fn set_is_eof_init(&mut self) {
        self.is_eof_init = true;
    }

    #[inline]
    pub fn eof(&self) -> Option<&Eof> {
        self.contract.bytecode.eof()
    }

    pub(crate) fn load_eof_code(&mut self, idx: usize, pc: usize) {
        let Bytecode::Eof(eof) = &self.contract.bytecode else {
            panic!("Expected EOF bytecode")
        };
        let Some(code) = eof.body.code(idx) else {
            panic!("Code not found")
        };
        self.bytecode = code.clone();
        self.instruction_pointer = unsafe { self.bytecode.as_ptr().add(pc) };
    }

    pub fn insert_create_outcome(&mut self, create_outcome: CreateOutcome) {
        self.instruction_result = InstructionResult::Continue;
        self.return_data_buffer = if create_outcome.instruction_result().is_revert() {
            create_outcome.output().to_owned()
        } else {
            Bytes::new()
        };

        match create_outcome.instruction_result() {
            return_ok!() => {
                push_b256!(self, create_outcome.address.unwrap_or_default().into_word());
                self.gas.erase_cost(create_outcome.gas().remaining());
                self.gas.record_refund(create_outcome.gas().refunded());
            }
            return_revert!() => {
                push!(self, U256::ZERO);
                self.gas.erase_cost(create_outcome.gas().remaining());
            }
            InstructionResult::FatalExternalError => {
                panic!("Fatal external error in insert_create_outcome");
            }
            _ => {
                push!(self, U256::ZERO);
            }
        }
    }

    pub fn insert_eofcreate_outcome(&mut self, create_outcome: EOFCreateOutcome) {
        self.return_data_buffer = if *create_outcome.instruction_result() == InstructionResult::Revert {
            create_outcome.output().to_owned()
        } else {
            Bytes::new()
        };

        match create_outcome.instruction_result() {
            InstructionResult::ReturnContract => {
                push_b256!(self, create_outcome.address.into_word());
                self.gas.erase_cost(create_outcome.gas().remaining());
                self.gas.record_refund(create_outcome.gas().refunded());
            }
            return_revert!() => {
                push!(self, U256::ZERO);
                self.gas.erase_cost(create_outcome.gas().remaining());
            }
            InstructionResult::FatalExternalError => {
                panic!("Fatal external error in insert_eofcreate_outcome");
            }
            _ => {
                push!(self, U256::ZERO);
            }
        }
    }

    pub fn insert_call_outcome(&mut self, shared_memory: &mut SharedMemory, call_outcome: CallOutcome) {
        self.instruction_result = InstructionResult::Continue;
        self.return_data_buffer.clone_from(call_outcome.output());

        let out_offset = call_outcome.memory_start();
        let out_len = call_outcome.memory_length();
        let target_len = min(out_len, self.return_data_buffer.len());

        match call_outcome.instruction_result() {
            return_ok!() => {
                self.gas.erase_cost(call_outcome.gas().remaining());
                self.gas.record_refund(call_outcome.gas().refunded());
                shared_memory.set(out_offset, &self.return_data_buffer[..target_len]);
                push!(self, U256::from(1));
            }
            return_revert!() => {
                self.gas.erase_cost(call_outcome.gas().remaining());
                shared_memory.set(out_offset, &self.return_data_buffer[..target_len]);
                push!(self, U256::ZERO);
            }
            InstructionResult::FatalExternalError => {
                panic!("Fatal external error in insert_call_outcome");
            }
            _ => {
                push!(self, U256::ZERO);
            }
        }
    }

    #[inline]
    pub fn current_opcode(&self) -> u8 {
        unsafe { *self.instruction_pointer }
    }

    #[inline]
    pub fn contract(&self) -> &Contract {
        &self.contract
    }

    #[inline]
    pub fn gas(&self) -> &Gas {
        &self.gas
    }

    #[inline]
    pub fn stack(&self) -> &Stack {
        &self.stack
    }

    #[inline]
    pub fn program_counter(&self) -> usize {
        unsafe { self.instruction_pointer.offset_from(self.bytecode.as_ptr()) as usize }
    }

    #[inline]
    pub(crate) fn step<FN, H: Host + ?Sized>(&mut self, instruction_table: &[FN; 256], host: &mut H)
    where
        FN: Fn(&mut Interpreter, &mut H),
    {
        let opcode = unsafe { *self.instruction_pointer };
        self.instruction_pointer = unsafe { self.instruction_pointer.offset(1) };
        (instruction_table[opcode as usize])(self, host)
    }

    pub fn take_memory(&mut self) -> SharedMemory {
        std::mem::replace(&mut self.shared_memory, EMPTY_SHARED_MEMORY)
    }

    pub fn run<FN, H: Host + ?Sized>(
        &mut self,
        shared_memory: SharedMemory,
        instruction_table: &[FN; 256],
        host: &mut H,
    ) -> InterpreterAction
    where
        FN: Fn(&mut Interpreter, &mut H),
    {
        self.next_action = InterpreterAction::None;
        self.shared_memory = shared_memory;
        
        while self.instruction_result == InstructionResult::Continue {
            self.step(instruction_table, host);
        }

        if self.next_action.is_some() {
            return std::mem::take(&mut self.next_action);
        }
        
        InterpreterAction::Return {
            result: InterpreterResult {
                result: self.instruction_result,
                output: Bytes::new(),
                gas: self.gas,
            },
        }
    }

    #[inline]
    #[must_use]
    pub fn resize_memory(&mut self, new_size: usize) -> bool {
        resize_memory(&mut self.shared_memory, &mut self.gas, new_size)
    }
}

impl InterpreterResult {
    #[inline]
    pub const fn is_ok(&self) -> bool {
        self.result.is_ok()
    }

    #[inline]
    pub const fn is_revert(&self) -> bool {
        self.result.is_revert()
    }

    #[inline]
    pub const fn is_error(&self) -> bool {
        self.result.is_error()
    }
}

#[inline(never)]
#[cold]
#[must_use]
pub fn resize_memory(memory: &mut SharedMemory, gas: &mut Gas, new_size: usize) -> bool {
    let new_words = num_words(new_size as u64);
    let new_cost = gas::memory_gas(new_words);
    let current_cost = memory.current_expansion_cost();
    let cost = new_cost - current_cost;
    let success = gas.record_cost(cost);
    if success {
        memory.resize((new_words as usize) * 32);
    }
    success
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{opcode::InstructionTable, DummyHost};
    use bcevm_primitives::CancunSpec;

    #[test]
    fn object_safety() {
        let mut interp = Interpreter::new(Contract::default(), u64::MAX, false);
        let mut host = DummyHost::default();
        let table: InstructionTable<DummyHost> = crate::opcode::make_instruction_table::<DummyHost, CancunSpec>();
        let _ = interp.run(EMPTY_SHARED_MEMORY, &table, &mut host);

        let host: &mut dyn Host = &mut host;
        let table: InstructionTable<dyn Host> = crate::opcode::make_instruction_table::<dyn Host, CancunSpec>();
        let _ = interp.run(EMPTY_SHARED_MEMORY, &table, host);
    }
}
