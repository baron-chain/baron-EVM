use crate::{primitives::U256, utilities::{left_pad, left_pad_vec, right_pad_vec, right_pad_with_offset}, Error, Precompile, PrecompileResult, PrecompileWithAddress};
use aurora_engine_modexp::modexp;
use core::cmp::{max, min};
use bcevm_primitives::Bytes;

pub const BYZANTIUM: PrecompileWithAddress = PrecompileWithAddress(crate::u64_to_address(5), Precompile::Standard(byzantium_run));
pub const BERLIN: PrecompileWithAddress = PrecompileWithAddress(crate::u64_to_address(5), Precompile::Standard(berlin_run));

pub fn byzantium_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    run_inner(input, gas_limit, 0, |a, b, c, d| byzantium_gas_calc(a, b, c, d))
}

pub fn berlin_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    run_inner(input, gas_limit, 200, |a, b, c, d| berlin_gas_calc(a, b, c, d))
}

pub fn calculate_iteration_count(exp_length: u64, exp_highp: &U256) -> u64 {
    let mut iteration_count = 0;
    if exp_length <= 32 && *exp_highp == U256::ZERO {
        iteration_count = 0;
    } else if exp_length <= 32 {
        iteration_count = exp_highp.bit_len() as u64 - 1;
    } else if exp_length > 32 {
        iteration_count = (8u64.saturating_mul(exp_length - 32)).saturating_add(max(1, exp_highp.bit_len() as u64) - 1);
    }
    max(iteration_count, 1)
}

pub fn run_inner<F>(input: &[u8], gas_limit: u64, min_gas: u64, calc_gas: F) -> PrecompileResult
where F: FnOnce(u64, u64, u64, &U256) -> u64 {
    if min_gas > gas_limit {
        return Err(Error::OutOfGas);
    }

    const HEADER_LENGTH: usize = 96;
    let base_len = U256::from_be_bytes(right_pad_with_offset::<32>(input, 0).into_owned());
    let exp_len = U256::from_be_bytes(right_pad_with_offset::<32>(input, 32).into_owned());
    let mod_len = U256::from_be_bytes(right_pad_with_offset::<32>(input, 64).into_owned());

    let Ok(base_len) = usize::try_from(base_len) else {
        return Err(Error::ModexpBaseOverflow);
    };
    let Ok(mod_len) = usize::try_from(mod_len) else {
        return Err(Error::ModexpModOverflow);
    };

    if base_len == 0 && mod_len == 0 {
        return Ok((min_gas, Bytes::new()));
    }

    let Ok(exp_len) = usize::try_from(exp_len) else {
        return Err(Error::ModexpModOverflow);
    };

    let exp_highp_len = min(exp_len, 32);
    let input = input.get(HEADER_LENGTH..).unwrap_or_default();

    let exp_highp = {
        let right_padded_highp = right_pad_with_offset::<32>(input, base_len);
        let out = left_pad::<32>(&right_padded_highp[..exp_highp_len]);
        U256::from_be_bytes(out.into_owned())
    };

    let gas_cost = calc_gas(base_len as u64, exp_len as u64, mod_len as u64, &exp_highp);
    if gas_cost > gas_limit {
        return Err(Error::OutOfGas);
    }

    let input_len = base_len.saturating_add(exp_len).saturating_add(mod_len);
    let input = right_pad_vec(input, input_len);
    let (base, input) = input.split_at(base_len);
    let (exponent, modulus) = input.split_at(exp_len);

    let output = modexp(base, exponent, modulus);
    Ok((gas_cost, left_pad_vec(&output, mod_len).into_owned().into()))
}

pub fn byzantium_gas_calc(base_len: u64, exp_len: u64, mod_len: u64, exp_highp: &U256) -> u64 {
    fn mul_complexity(x: u64) -> U256 {
        if x <= 64 {
            U256::from(x * x)
        } else if x <= 1_024 {
            U256::from(x * x / 4 + 96 * x - 3_072)
        } else {
            let x = U256::from(x);
            let x_sq = x * x;
            x_sq / U256::from(16) + U256::from(480) * x - U256::from(199_680)
        }
    }

    let mul = mul_complexity(core::cmp::max(mod_len, base_len));
    let iter_count = U256::from(calculate_iteration_count(exp_len, exp_highp));
    let gas = (mul * iter_count) / U256::from(20);
    gas.saturating_to()
}

pub fn berlin_gas_calc(base_length: u64, exp_length: u64, mod_length: u64, exp_highp: &U256) -> u64 {
    fn calculate_multiplication_complexity(base_length: u64, mod_length: u64) -> U256 {
        let max_length = max(base_length, mod_length);
        let mut words = max_length / 8;
        if max_length % 8 > 0 {
            words += 1;
        }
        let words = U256::from(words);
        words * words
    }

    let multiplication_complexity = calculate_multiplication_complexity(base_length, mod_length);
    let iteration_count = calculate_iteration_count(exp_length, exp_highp);
    let gas = (multiplication_complexity * U256::from(iteration_count)) / U256::from(3);
    max(200, gas.saturating_to())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcevm_primitives::hex;

    struct Test {
        input: &'static str,
        expected: &'static str,
        name: &'static str,
    }

    const TESTS: [Test; 19] = [ /* ... Test cases ... */ ];

    const BYZANTIUM_GAS: [u64; 19] = [
        360_217, 13_056, 13_056, 13_056, 204, 204, 3_276, 665, 665, 10_649, 1_894, 1_894, 30_310,
        5_580, 5_580, 89_292, 17_868, 17_868, 285_900,
    ];

    const BERLIN_GAS: [u64; 19] = [
        44_954, 1_360, 1_360, 1_360, 200, 200, 341, 200, 200, 1_365, 341, 341, 5_461, 1_365, 1_365,
        21_845, 5_461, 5_461, 87_381,
    ];

    #[test]
    fn test_byzantium_modexp_gas() {
        for (test, &test_gas) in TESTS.iter().zip(BYZANTIUM_GAS.iter()) {
            let input = hex::decode(test.input).unwrap().into();
            let res = byzantium_run(&input, 100_000_000).unwrap();
            let expected = hex::decode(test.expected).unwrap();
            assert_eq!(res.0, test_gas, "used gas not matching for test: {}", test.name);
            assert_eq!(res.1, expected, "test:{}", test.name);
        }
    }

    #[test]
    fn test_berlin_modexp_gas() {
        for (test, &test_gas) in TESTS.iter().zip(BERLIN_GAS.iter()) {
            let input = hex::decode(test.input).unwrap().into();
            let res = berlin_run(&input, 100_000_000).unwrap();
            let expected = hex::decode(test.expected).unwrap();
            assert_eq!(res.0, test_gas, "used gas not matching for test: {}", test.name);
            assert_eq!(res.1, expected, "test:{}", test.name);
        }
    }

    #[test]
    fn test_berlin_modexp_empty_input() {
        let res = berlin_run(&Bytes::new(), 100_000).unwrap();
        let expected: Vec<u8> = Vec::new();
        assert_eq!(res.1, expected)
    }
}
