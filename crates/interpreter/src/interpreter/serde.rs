use crate::{
    Contract, FunctionStack, Gas, InstructionResult, InterpreterAction, SharedMemory, Stack,
};
use super::Interpreter;
use bcevm_primitives::Bytes;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{self, MapAccess, Visitor};
use serde::ser::SerializeStruct;
use std::fmt;

impl Serialize for Interpreter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Interpreter", 13)?;
        state.serialize_field("program_counter", &self.program_counter())?;
        state.serialize_field("gas", &self.gas)?;
        state.serialize_field("contract", &self.contract)?;
        state.serialize_field("instruction_result", &self.instruction_result)?;
        state.serialize_field("bytecode", &self.bytecode)?;
        state.serialize_field("is_eof", &self.is_eof)?;
        state.serialize_field("is_eof_init", &self.is_eof_init)?;
        state.serialize_field("shared_memory", &self.shared_memory)?;
        state.serialize_field("stack", &self.stack)?;
        state.serialize_field("function_stack", &self.function_stack)?;
        state.serialize_field("return_data_buffer", &self.return_data_buffer)?;
        state.serialize_field("is_static", &self.is_static)?;
        state.serialize_field("next_action", &self.next_action)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Interpreter {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            ProgramCounter,
            Gas,
            Contract,
            InstructionResult,
            Bytecode,
            IsEof,
            IsEofInit,
            SharedMemory,
            Stack,
            FunctionStack,
            ReturnDataBuffer,
            IsStatic,
            NextAction,
        }

        struct InterpreterVisitor;

        impl<'de> Visitor<'de> for InterpreterVisitor {
            type Value = Interpreter;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Interpreter")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Interpreter, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut program_counter = None;
                let mut gas = None;
                let mut contract = None;
                let mut instruction_result = None;
                let mut bytecode = None;
                let mut is_eof = None;
                let mut is_eof_init = None;
                let mut shared_memory = None;
                let mut stack = None;
                let mut function_stack = None;
                let mut return_data_buffer = None;
                let mut is_static = None;
                let mut next_action = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::ProgramCounter => { program_counter = Some(map.next_value()?); }
                        Field::Gas => { gas = Some(map.next_value()?); }
                        Field::Contract => { contract = Some(map.next_value()?); }
                        Field::InstructionResult => { instruction_result = Some(map.next_value()?); }
                        Field::Bytecode => { bytecode = Some(map.next_value()?); }
                        Field::IsEof => { is_eof = Some(map.next_value()?); }
                        Field::IsEofInit => { is_eof_init = Some(map.next_value()?); }
                        Field::SharedMemory => { shared_memory = Some(map.next_value()?); }
                        Field::Stack => { stack = Some(map.next_value()?); }
                        Field::FunctionStack => { function_stack = Some(map.next_value()?); }
                        Field::ReturnDataBuffer => { return_data_buffer = Some(map.next_value()?); }
                        Field::IsStatic => { is_static = Some(map.next_value()?); }
                        Field::NextAction => { next_action = Some(map.next_value()?); }
                    }
                }

                let program_counter = program_counter.ok_or_else(|| de::Error::missing_field("program_counter"))?;
                let gas = gas.ok_or_else(|| de::Error::missing_field("gas"))?;
                let contract = contract.ok_or_else(|| de::Error::missing_field("contract"))?;
                let instruction_result = instruction_result.ok_or_else(|| de::Error::missing_field("instruction_result"))?;
                let bytecode = bytecode.ok_or_else(|| de::Error::missing_field("bytecode"))?;
                let is_eof = is_eof.ok_or_else(|| de::Error::missing_field("is_eof"))?;
                let is_eof_init = is_eof_init.ok_or_else(|| de::Error::missing_field("is_eof_init"))?;
                let shared_memory = shared_memory.ok_or_else(|| de::Error::missing_field("shared_memory"))?;
                let stack = stack.ok_or_else(|| de::Error::missing_field("stack"))?;
                let function_stack = function_stack.ok_or_else(|| de::Error::missing_field("function_stack"))?;
                let return_data_buffer = return_data_buffer.ok_or_else(|| de::Error::missing_field("return_data_buffer"))?;
                let is_static = is_static.ok_or_else(|| de::Error::missing_field("is_static"))?;
                let next_action = next_action.ok_or_else(|| de::Error::missing_field("next_action"))?;

                if program_counter < 0 || program_counter >= bytecode.len() as isize {
                    return Err(de::Error::custom("program_counter index out of range"));
                }

                let instruction_pointer = unsafe { bytecode.as_ptr().offset(program_counter) };

                Ok(Interpreter {
                    instruction_pointer,
                    gas,
                    contract,
                    instruction_result,
                    bytecode,
                    is_eof,
                    is_eof_init,
                    shared_memory,
                    stack,
                    function_stack,
                    return_data_buffer,
                    is_static,
                    next_action,
                })
            }
        }

        const FIELDS: &[&str] = &[
            "program_counter", "gas", "contract", "instruction_result", "bytecode",
            "is_eof", "is_eof_init", "shared_memory", "stack", "function_stack",
            "return_data_buffer", "is_static", "next_action",
        ];

        deserializer.deserialize_struct("Interpreter", FIELDS, InterpreterVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serde() {
        let interp = Interpreter::new(Contract::default(), u64::MAX, false);
        let serialized = bincode::serialize(&interp).unwrap();
        let de: Interpreter = bincode::deserialize(&serialized).unwrap();
        assert_eq!(interp.program_counter(), de.program_counter());
    }
}
