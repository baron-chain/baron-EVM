use super::calc_linear_cost_u32;
use crate::{Error, Precompile, PrecompileResult, PrecompileWithAddress};
use bcevm_primitives::Bytes;
use sha2::Digest;

pub const SHA256: PrecompileWithAddress = PrecompileWithAddress(crate::u64_to_address(2), Precompile::Standard(sha256_run));
pub const RIPEMD160: PrecompileWithAddress = PrecompileWithAddress(crate::u64_to_address(3), Precompile::Standard(ripemd160_run));

pub fn sha256_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let cost = calc_linear_cost_u32(input.len(), 60, 12);
    if cost > gas_limit {
        Err(Error::OutOfGas)
    } else {
        Ok((cost, sha2::Sha256::digest(input).to_vec().into()))
    }
}

pub fn ripemd160_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    let gas_used = calc_linear_cost_u32(input.len(), 600, 120);
    if gas_used > gas_limit {
        Err(Error::OutOfGas)
    } else {
        let mut output = [0u8; 32];
        ripemd::Ripemd160::digest(input).copy_to_slice(&mut output[12..]);
        Ok((gas_used, output.to_vec().into()))
    }
}
