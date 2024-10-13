use crate::{b256, B256, BLOB_GASPRICE_UPDATE_FRACTION, MIN_BLOB_GASPRICE, TARGET_BLOB_GAS_PER_BLOCK};
pub use alloy_primitives::keccak256;

pub const KECCAK_EMPTY: B256 = b256!("c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470");

#[inline]
pub fn calc_excess_blob_gas(parent_excess_blob_gas: u64, parent_blob_gas_used: u64) -> u64 {
    (parent_excess_blob_gas + parent_blob_gas_used).saturating_sub(TARGET_BLOB_GAS_PER_BLOCK)
}

#[inline]
pub fn calc_blob_gasprice(excess_blob_gas: u64) -> u128 {
    fake_exponential(
        MIN_BLOB_GASPRICE,
        excess_blob_gas,
        BLOB_GASPRICE_UPDATE_FRACTION,
    )
}

#[inline]
pub fn fake_exponential(factor: u64, numerator: u64, denominator: u64) -> u128 {
    debug_assert!(denominator != 0, "attempt to divide by zero");
    let (factor, numerator, denominator) = (factor as u128, numerator as u128, denominator as u128);

    let mut output = 0;
    let mut numerator_accum = factor * denominator;
    let mut i = 1;

    while numerator_accum > 0 {
        output += numerator_accum;
        numerator_accum = (numerator_accum * numerator) / (denominator * i);
        i += 1;
    }
    output / denominator
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GAS_PER_BLOB;

    #[test]
    fn test_calc_excess_blob_gas() {
        let test_cases = [
            (0, 0, 0),
            (0, TARGET_BLOB_GAS_PER_BLOCK / GAS_PER_BLOB, 0),
            (0, (TARGET_BLOB_GAS_PER_BLOCK / GAS_PER_BLOB) + 1, GAS_PER_BLOB),
            (1, (TARGET_BLOB_GAS_PER_BLOCK / GAS_PER_BLOB) + 2, 2 * GAS_PER_BLOB + 1),
            (TARGET_BLOB_GAS_PER_BLOCK, (TARGET_BLOB_GAS_PER_BLOCK / GAS_PER_BLOB) - 1, TARGET_BLOB_GAS_PER_BLOCK - GAS_PER_BLOB),
            (GAS_PER_BLOB - 1, (TARGET_BLOB_GAS_PER_BLOCK / GAS_PER_BLOB) - 1, 0),
        ];

        for (excess, blobs, expected) in test_cases.iter() {
            let actual = calc_excess_blob_gas(*excess, blobs * GAS_PER_BLOB);
            assert_eq!(actual, *expected, "test case: ({}, {}, {})", excess, blobs, expected);
        }
    }

    #[test]
    fn test_calc_blob_fee() {
        let test_cases = [
            (0, 1),
            (2314057, 1),
            (2314058, 2),
            (10 * 1024 * 1024, 23),
            (148099578, 18446739238971471609),
            (148099579, 18446744762204311910),
            (161087488, 902580055246494526580),
        ];

        for (excess, expected) in test_cases.iter() {
            let actual = calc_blob_gasprice(*excess);
            assert_eq!(actual, *expected, "test case: ({}, {})", excess, expected);
        }
    }

    #[test]
    fn test_fake_exp() {
        let test_cases = [
            (1, 0, 1, 1),
            (38493, 0, 1000, 38493),
            (1, 2, 1, 6),
            (1, 4, 1, 49),
            (10, 8, 2, 542),
            (1, 50000000, 2225652, 5709098764),
            (1, 380928, BLOB_GASPRICE_UPDATE_FRACTION, 1),
        ];

        for (factor, numerator, denominator, expected) in test_cases.iter() {
            let actual = fake_exponential(*factor, *numerator, *denominator);
            assert_eq!(actual, *expected, "test case: ({}, {}, {}, {})", factor, numerator, denominator, expected);
        }
    }
}
