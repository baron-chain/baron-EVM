use std::vec::Vec;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionReturnFrame {
    pub idx: usize,
    pub pc: usize,
}

impl FunctionReturnFrame {
    #[inline]
    pub fn new(idx: usize, pc: usize) -> Self {
        Self { idx, pc }
    }
}

#[derive(Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct FunctionStack {
    return_stack: Vec<FunctionReturnFrame>,
    current_code_idx: usize,
}

impl FunctionStack {
    #[inline]
    pub fn new() -> Self {
        Self {
            return_stack: Vec::new(),
            current_code_idx: 0,
        }
    }

    #[inline]
    pub fn push(&mut self, program_counter: usize, new_idx: usize) {
        self.return_stack.push(FunctionReturnFrame {
            idx: self.current_code_idx,
            pc: program_counter,
        });
        self.current_code_idx = new_idx;
    }

    #[inline]
    pub fn return_stack_len(&self) -> usize {
        self.return_stack.len()
    }

    #[inline]
    pub fn pop(&mut self) -> Option<FunctionReturnFrame> {
        self.return_stack.pop().map(|frame| {
            self.current_code_idx = frame.idx;
            frame
        })
    }

    #[inline]
    pub fn set_current_code_idx(&mut self, idx: usize) {
        self.current_code_idx = idx;
    }

    #[inline]
    pub fn current_code_idx(&self) -> usize {
        self.current_code_idx
    }
}
