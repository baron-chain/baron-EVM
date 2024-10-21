use bcevm_primitives::{eof::EofDecodeError, HashSet};
use crate::{
    instructions::utility::{read_i16, read_u16},
    opcode,
    primitives::{
        bitvec::prelude::{bitvec, BitVec, Lsb0},
        eof::TypesSection,
        legacy::JumpTable,
        Bytecode, Bytes, Eof, LegacyAnalyzedBytecode,
    },
    OPCODE_INFO_JUMPTABLE, STACK_LIMIT,
};
use std::{sync::Arc, vec, vec::Vec};

const EOF_NON_RETURNING_FUNCTION: u8 = 0x80;

#[inline]
pub fn to_analysed(bytecode: Bytecode) -> Bytecode {
    match bytecode {
        Bytecode::LegacyRaw(bytecode) => {
            let len = bytecode.len();
            let mut padded_bytecode = Vec::with_capacity(len + 33);
            padded_bytecode.extend_from_slice(&bytecode);
            padded_bytecode.resize(len + 33, 0);
            let bytes = Bytes::from(padded_bytecode);
            let jump_table = analyze(bytes.as_ref());
            Bytecode::LegacyAnalyzed(LegacyAnalyzedBytecode::new(bytes, len, jump_table))
        }
        _ => bytecode,
    }
}

fn analyze(code: &[u8]) -> JumpTable {
    let mut jumps: BitVec<u8> = bitvec![u8, Lsb0; 0; code.len()];
    let mut iterator = code.as_ptr();
    let end = unsafe { iterator.add(code.len()) };
    
    while iterator < end {
        let opcode = unsafe { *iterator };
        if opcode == opcode::JUMPDEST {
            let offset = unsafe { iterator.offset_from(code.as_ptr()) as usize };
            jumps.set(offset, true);
            iterator = unsafe { iterator.add(1) };
        } else {
            let push_offset = opcode.wrapping_sub(opcode::PUSH1);
            iterator = unsafe { 
                iterator.add(if push_offset < 32 { push_offset as usize + 2 } else { 1 })
            };
        }
    }
    
    JumpTable(Arc::new(jumps))
}

pub fn validate_raw_eof(bytecode: Bytes) -> Result<Eof, EofError> {
    let eof = Eof::decode(bytecode)?;
    validate_eof(&eof)?;
    Ok(eof)
}

pub fn validate_eof(eof: &Eof) -> Result<(), EofError> {
    let mut queue = vec![eof.clone()];
    while let Some(eof) = queue.pop() {
        validate_eof_codes(&eof)?;
        for container in eof.body.container_section {
            queue.push(Eof::decode(container)?);
        }
    }
    Ok(())
}

pub fn validate_eof_codes(eof: &Eof) -> Result<(), EofValidationError> {
    let mut queued_codes = vec![false; eof.body.code_section.len()];
    if eof.body.code_section.len() != eof.body.types_section.len() {
        return Err(EofValidationError::InvalidTypesSection);
    }
    if eof.body.code_section.is_empty() {
        return Err(EofValidationError::NoCodeSections);
    }
    queued_codes[0] = true;
    
    let first_types = &eof.body.types_section[0];
    if first_types.inputs != 0 || first_types.outputs != EOF_NON_RETURNING_FUNCTION {
        return Err(EofValidationError::InvalidTypesSection);
    }
    
    let mut queue = vec![0];
    while let Some(index) = queue.pop() {
        let code = &eof.body.code_section[index];
        let accessed_codes = validate_eof_code(
            code,
            eof.header.data_size as usize,
            index,
            eof.body.container_section.len(),
            &eof.body.types_section,
        )?;
        
        for i in accessed_codes {
            if !queued_codes[i] {
                queued_codes[i] = true;
                queue.push(i);
            }
        }
    }
    
    if queued_codes.iter().any(|&x| !x) {
        return Err(EofValidationError::CodeSectionNotAccessed);
    }
    
    Ok(())
}

// EofError and EofValidationError definitions remain the same

pub fn validate_eof_code(
    code: &[u8],
    data_size: usize,
    this_types_index: usize,
    num_of_containers: usize,
    types: &[TypesSection],
) -> Result<HashSet<usize>, EofValidationError> {
    // InstructionInfo struct definition remains the same

    let mut accessed_codes = HashSet::new();
    let this_types = &types[this_types_index];
    let mut jumps = vec![InstructionInfo::default(); code.len()];
    let mut is_after_termination = false;
    let mut next_smallest = this_types.inputs as i32;
    let mut next_biggest = this_types.inputs as i32;

    let mut i = 0;
    while i < code.len() {
        let op = code[i];
        let Some(opcode) = OPCODE_INFO_JUMPTABLE.get(op as usize) else {
            return Err(EofValidationError::UnknownOpcode);
        };

        if opcode.is_disabled_in_eof() {
            return Err(EofValidationError::OpcodeDisabled);
        }

        let this_instruction = &mut jumps[i];
        if !is_after_termination {
            this_instruction.smallest = this_instruction.smallest.min(next_smallest);
            this_instruction.biggest = this_instruction.biggest.max(next_biggest);
        }

        let this_instruction = *this_instruction;

        if is_after_termination && !this_instruction.is_jumpdest {
            return Err(EofValidationError::InstructionNotForwardAccessed);
        }
        is_after_termination = opcode.is_terminating();

        if opcode.immediate_size() != 0 {
            if i + opcode.immediate_size() as usize >= code.len() {
                return Err(EofValidationError::MissingImmediateBytes);
            }
            for imm in 1..=opcode.immediate_size() as usize {
                jumps[i + imm].mark_as_immediate()?;
            }
        }

        let mut stack_io_diff = opcode.io_diff() as i32;
        let mut stack_requirement = opcode.inputs() as i32;
        let mut rjumpv_additional_immediates = 0;
        let mut absolute_jumpdest = vec![];

        // Match block for specific opcodes remains largely the same
        // Consider optimizing this part if there are any repetitive patterns

        if stack_requirement > this_instruction.smallest {
            return Err(EofValidationError::StackUnderflow);
        }

        next_smallest = this_instruction.smallest + stack_io_diff;
        next_biggest = this_instruction.biggest + stack_io_diff;

        for absolute_jump in absolute_jumpdest {
            if absolute_jump < 0 {
                return Err(EofValidationError::JumpUnderflow);
            }
            if absolute_jump >= code.len() as isize {
                return Err(EofValidationError::JumpOverflow);
            }
            let absolute_jump = absolute_jump as usize;
            let target_jump = &mut jumps[absolute_jump];
            if target_jump.is_immediate {
                return Err(EofValidationError::BackwardJumpToImmediateBytes);
            }
            target_jump.is_jumpdest = true;
            if absolute_jump <= i {
                if target_jump.biggest != next_biggest || target_jump.smallest != next_smallest {
                    return Err(EofValidationError::BackwardJumpBiggestNumMismatch);
                }
            } else {
                target_jump.smallest = target_jump.smallest.min(next_smallest);
                target_jump.biggest = target_jump.biggest.max(next_biggest);
            }
        }

        i += 1 + opcode.immediate_size() as usize + rjumpv_additional_immediates;
    }

    if !is_after_termination {
        return Err(EofValidationError::LastInstructionNotTerminating);
    }

    let max_stack_requirement = jumps.iter().map(|opcode| opcode.biggest).max().unwrap_or(0);
    if max_stack_requirement != types[this_types_index].max_stack_size as i32 {
        return Err(EofValidationError::MaxStackMismatch);
    }

    Ok(accessed_codes)
}

#[cfg(test)]
mod test {
    use super::*;
    use bcevm_primitives::hex;

    #[test]
    fn test1() {
        let err = validate_raw_eof(hex!("ef0001010004020001000704000000008000016000e200fffc00").into());
        assert!(err.is_err());
    }

    #[test]
    fn test2() {
        let err = validate_raw_eof(hex!("ef000101000c02000300040004000204000000008000020002000100010001e30001005fe500025fe4").into());
        assert!(err.is_ok());
    }

    #[test]
    fn test3() {
        let err = validate_raw_eof(hex!("ef000101000c02000300040008000304000000008000020002000503010003e30001005f5f5f5f5fe500025050e4").into());
        assert_eq!(err, Err(EofError::Validation(EofValidationError::JUMPFStackHigherThanOutputs)));
    }
}
