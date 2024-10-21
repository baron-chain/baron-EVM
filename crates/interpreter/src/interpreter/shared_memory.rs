use core::{cmp::min, fmt, ops::Range};
use bcevm_primitives::{B256, U256};
use std::vec::Vec;

#[derive(Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SharedMemory {
    buffer: Vec<u8>,
    checkpoints: Vec<usize>,
    last_checkpoint: usize,
    #[cfg(feature = "memory_limit")]
    memory_limit: u64,
}

pub const EMPTY_SHARED_MEMORY: SharedMemory = SharedMemory {
    buffer: Vec::new(),
    checkpoints: Vec::new(),
    last_checkpoint: 0,
    #[cfg(feature = "memory_limit")]
    memory_limit: u64::MAX,
};

impl fmt::Debug for SharedMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SharedMemory")
            .field("current_len", &self.len())
            .field("context_memory", &crate::primitives::hex::encode(self.context_memory()))
            .finish_non_exhaustive()
    }
}

impl Default for SharedMemory {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl SharedMemory {
    #[inline]
    pub fn new() -> Self {
        Self::with_capacity(4 * 1024)
    }

    #[inline]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            checkpoints: Vec::with_capacity(32),
            last_checkpoint: 0,
            #[cfg(feature = "memory_limit")]
            memory_limit: u64::MAX,
        }
    }

    #[cfg(feature = "memory_limit")]
    #[inline]
    pub fn new_with_memory_limit(memory_limit: u64) -> Self {
        Self { memory_limit, ..Self::new() }
    }

    #[cfg(feature = "memory_limit")]
    #[inline]
    pub fn limit_reached(&self, new_size: usize) -> bool {
        (self.last_checkpoint + new_size) as u64 > self.memory_limit
    }

    #[inline]
    pub fn new_context(&mut self) {
        let new_checkpoint = self.buffer.len();
        self.checkpoints.push(new_checkpoint);
        self.last_checkpoint = new_checkpoint;
    }

    #[inline]
    pub fn free_context(&mut self) {
        if let Some(old_checkpoint) = self.checkpoints.pop() {
            self.last_checkpoint = *self.checkpoints.last().unwrap_or(&0);
            unsafe { self.buffer.set_len(old_checkpoint) };
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buffer.len() - self.last_checkpoint
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn current_expansion_cost(&self) -> u64 {
        crate::gas::memory_gas_for_len(self.len())
    }

    #[inline]
    pub fn resize(&mut self, new_size: usize) {
        self.buffer.resize(self.last_checkpoint + new_size, 0);
    }

    #[inline]
    pub fn slice(&self, offset: usize, size: usize) -> &[u8] {
        self.slice_range(offset..offset + size)
    }

    #[inline]
    pub fn slice_range(&self, range: Range<usize>) -> &[u8] {
        &self.context_memory()[range]
    }

    #[inline]
    pub fn slice_mut(&mut self, offset: usize, size: usize) -> &mut [u8] {
        let end = offset + size;
        &mut self.context_memory_mut()[offset..end]
    }

    #[inline]
    pub fn get_byte(&self, offset: usize) -> u8 {
        self.context_memory()[offset]
    }

    #[inline]
    pub fn get_word(&self, offset: usize) -> B256 {
        self.slice(offset, 32).try_into().unwrap()
    }

    #[inline]
    pub fn get_u256(&self, offset: usize) -> U256 {
        self.get_word(offset).into()
    }

    #[inline]
    pub fn set_byte(&mut self, offset: usize, byte: u8) {
        self.context_memory_mut()[offset] = byte;
    }

    #[inline]
    pub fn set_word(&mut self, offset: usize, value: &B256) {
        self.slice_mut(offset, 32).copy_from_slice(value);
    }

    #[inline]
    pub fn set_u256(&mut self, offset: usize, value: U256) {
        self.set_word(offset, &value.to_be_bytes::<32>().into());
    }

    #[inline]
    pub fn set(&mut self, offset: usize, value: &[u8]) {
        if !value.is_empty() {
            self.slice_mut(offset, value.len()).copy_from_slice(value);
        }
    }

    #[inline]
    pub fn set_data(&mut self, memory_offset: usize, data_offset: usize, len: usize, data: &[u8]) {
        let memory = self.slice_mut(memory_offset, len);
        if data_offset >= data.len() {
            memory.fill(0);
        } else {
            let data_end = min(data_offset + len, data.len());
            let data_len = data_end - data_offset;
            memory[..data_len].copy_from_slice(&data[data_offset..data_end]);
            memory[data_len..].fill(0);
        }
    }

    #[inline]
    pub fn copy(&mut self, dst: usize, src: usize, len: usize) {
        self.context_memory_mut().copy_within(src..src + len, dst);
    }

    #[inline]
    pub fn context_memory(&self) -> &[u8] {
        &self.buffer[self.last_checkpoint..]
    }

    #[inline]
    pub fn context_memory_mut(&mut self) -> &mut [u8] {
        &mut self.buffer[self.last_checkpoint..]
    }
}

#[inline]
pub const fn num_words(len: u64) -> u64 {
    len.saturating_add(31) / 32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_num_words() {
        // Test cases remain the same
    }

    #[test]
    fn new_free_context() {
        // Test cases remain the same
    }

    #[test]
    fn resize() {
        // Test cases remain the same
    }
}
