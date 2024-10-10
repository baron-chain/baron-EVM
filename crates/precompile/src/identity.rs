use super::calc_linear_cost_u32;
use crate::{Error, Precompile, PrecompileResult, PrecompileWithAddress};
use bcevm_primitives::Bytes;

pub const FUN: PrecompileWithAddress = PrecompileWithAddress(crate::u64_to_address(4), Precompile::Standard(identity_run));
pub const IDENTITY_BASE: u64 = 15;
pub const IDENTITY_PER_WORD: u64 = 3;

pub fn identity_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let gas_used = calc_linear_cost_u32(input.len(), IDENTITY_BASE, IDENTITY_PER_WORD);
    if gas_used > gas_limit {
        Err(Error::OutOfGas)
    } else {
        Ok((gas_used, input.clone()))
    }
}
