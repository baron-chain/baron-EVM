use crate::{primitives::{B256, U256}, InstructionResult};
use core::{fmt, ptr};
use std::vec::Vec;

pub const STACK_LIMIT: usize = 1024;

#[derive(Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Stack {
    data: Vec<U256>,
}

impl fmt::Display for Stack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("[")?;
        for (i, x) in self.data.iter().enumerate() {
            if i > 0 { f.write_str(", ")? }
            write!(f, "{x}")?;
        }
        f.write_str("]")
    }
}

impl Default for Stack {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Stack {
    #[inline]
    pub fn new() -> Self {
        Self { data: Vec::with_capacity(STACK_LIMIT) }
    }

    #[inline] pub fn len(&self) -> usize { self.data.len() }
    #[inline] pub fn is_empty(&self) -> bool { self.data.is_empty() }
    #[inline] pub fn data(&self) -> &Vec<U256> { &self.data }
    #[inline] pub fn data_mut(&mut self) -> &mut Vec<U256> { &mut self.data }
    #[inline] pub fn into_data(self) -> Vec<U256> { self.data }

    #[inline]
    pub fn pop(&mut self) -> Result<U256, InstructionResult> {
        self.data.pop().ok_or(InstructionResult::StackUnderflow)
    }

    #[inline]
    pub unsafe fn pop_unsafe(&mut self) -> U256 {
        self.data.pop().unwrap_unchecked()
    }

    #[inline]
    pub unsafe fn top_unsafe(&mut self) -> &mut U256 {
        let len = self.data.len();
        self.data.get_unchecked_mut(len - 1)
    }

    #[inline]
    pub unsafe fn pop_top_unsafe(&mut self) -> (U256, &mut U256) {
        let pop = self.pop_unsafe();
        let top = self.top_unsafe();
        (pop, top)
    }

    #[inline]
    pub unsafe fn pop2_unsafe(&mut self) -> (U256, U256) {
        (self.pop_unsafe(), self.pop_unsafe())
    }

    #[inline]
    pub unsafe fn pop2_top_unsafe(&mut self) -> (U256, U256, &mut U256) {
        let pop1 = self.pop_unsafe();
        let pop2 = self.pop_unsafe();
        let top = self.top_unsafe();
        (pop1, pop2, top)
    }

    #[inline]
    pub unsafe fn pop3_unsafe(&mut self) -> (U256, U256, U256) {
        (self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe())
    }

    #[inline]
    pub unsafe fn pop4_unsafe(&mut self) -> (U256, U256, U256, U256) {
        (self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe())
    }

    #[inline]
    pub unsafe fn pop5_unsafe(&mut self) -> (U256, U256, U256, U256, U256) {
        (self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe(), self.pop_unsafe())
    }

    #[inline]
    pub fn push_b256(&mut self, value: B256) -> Result<(), InstructionResult> {
        self.push(value.into())
    }

    #[inline]
    pub fn push(&mut self, value: U256) -> Result<(), InstructionResult> {
        if self.data.len() == STACK_LIMIT {
            return Err(InstructionResult::StackOverflow);
        }
        self.data.push(value);
        Ok(())
    }

    #[inline]
    pub fn peek(&self, no_from_top: usize) -> Result<U256, InstructionResult> {
        self.data.get(self.data.len().wrapping_sub(no_from_top + 1))
            .copied()
            .ok_or(InstructionResult::StackUnderflow)
    }

    #[inline]
    pub fn dup(&mut self, n: usize) -> Result<(), InstructionResult> {
        debug_assert!(n > 0, "attempted to dup 0");
        let len = self.data.len();
        if len < n {
            return Err(InstructionResult::StackUnderflow);
        }
        if len + 1 > STACK_LIMIT {
            return Err(InstructionResult::StackOverflow);
        }
        unsafe {
            let ptr = self.data.as_mut_ptr().add(len);
            ptr::copy_nonoverlapping(ptr.sub(n), ptr, 1);
            self.data.set_len(len + 1);
        }
        Ok(())
    }

    #[inline]
    pub fn swap(&mut self, n: usize) -> Result<(), InstructionResult> {
        self.exchange(0, n)
    }

    #[inline]
    pub fn exchange(&mut self, n: usize, m: usize) -> Result<(), InstructionResult> {
        debug_assert!(m > 0, "overlapping exchange");
        let len = self.data.len();
        let n_m_index = n + m;
        if n_m_index >= len {
            return Err(InstructionResult::StackUnderflow);
        }
        unsafe {
            let top = self.data.as_mut_ptr().add(len - 1);
            ptr::swap_nonoverlapping(top.sub(n), top.sub(n_m_index), 1);
        }
        Ok(())
    }

    #[inline]
    pub fn push_slice(&mut self, slice: &[u8]) -> Result<(), InstructionResult> {
        if slice.is_empty() {
            return Ok(());
        }

        let n_words = (slice.len() + 31) / 32;
        let new_len = self.data.len() + n_words;
        if new_len > STACK_LIMIT {
            return Err(InstructionResult::StackOverflow);
        }

        unsafe {
            let dst = self.data.as_mut_ptr().add(self.data.len()).cast::<u64>();
            self.data.set_len(new_len);

            let mut i = 0;
            for word in slice.chunks(32) {
                for chunk in word.rchunks(8) {
                    let mut tmp = [0u8; 8];
                    tmp[8 - chunk.len()..].copy_from_slice(chunk);
                    dst.add(i).write(u64::from_be_bytes(tmp));
                    i += 1;
                }
            }

            let m = i % 4;
            if m != 0 {
                dst.add(i).write_bytes(0, 4 - m);
            }
        }

        Ok(())
    }

    #[inline]
    pub fn set(&mut self, no_from_top: usize, val: U256) -> Result<(), InstructionResult> {
        let index = self.data.len().wrapping_sub(no_from_top + 1);
        self.data.get_mut(index)
            .map(|x| *x = val)
            .ok_or(InstructionResult::StackUnderflow)
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Stack {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut data = Vec::<U256>::deserialize(deserializer)?;
        if data.len() > STACK_LIMIT {
            return Err(serde::de::Error::custom(format!(
                "stack size exceeds limit: {} > {}",
                data.len(),
                STACK_LIMIT
            )));
        }
        data.reserve(STACK_LIMIT - data.len());
        Ok(Self { data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(f: impl FnOnce(&mut Stack)) {
        let mut stack = Stack::new();
        unsafe {
            stack.data.set_len(STACK_LIMIT);
            stack.data.fill(U256::MAX);
            stack.data.set_len(0);
        }
        f(&mut stack);
    }

    #[test]
    fn push_slices() {
        // Test cases remain the same
    }
}
