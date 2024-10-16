use crate::primitives::{address, db::Database, Address, SpecId, U256};
use core::ops::Mul;

const ZERO_BYTE_COST: u64 = 4;
const NON_ZERO_BYTE_COST: u64 = 16;
const BASE_FEE_SCALAR_OFFSET: usize = 16;
const BLOB_BASE_FEE_SCALAR_OFFSET: usize = 20;

const L1_BASE_FEE_SLOT: U256 = U256::from_limbs([1, 0, 0, 0]);
const L1_OVERHEAD_SLOT: U256 = U256::from_limbs([5, 0, 0, 0]);
const L1_SCALAR_SLOT: U256 = U256::from_limbs([6, 0, 0, 0]);
const ECOTONE_L1_BLOB_BASE_FEE_SLOT: U256 = U256::from_limbs([7, 0, 0, 0]);
const ECOTONE_L1_FEE_SCALARS_SLOT: U256 = U256::from_limbs([3, 0, 0, 0]);

const EMPTY_SCALARS: [u8; 8] = [0; 8];

pub const L1_FEE_RECIPIENT: Address = address!("420000000000000000000000000000000000001A");
pub const BASE_FEE_RECIPIENT: Address = address!("4200000000000000000000000000000000000019");
pub const L1_BLOCK_CONTRACT: Address = address!("4200000000000000000000000000000000000015");

#[derive(Clone, Debug, Default)]
pub struct L1BlockInfo {
    pub l1_base_fee: U256,
    pub l1_fee_overhead: Option<U256>,
    pub l1_base_fee_scalar: U256,
    pub l1_blob_base_fee: Option<U256>,
    pub l1_blob_base_fee_scalar: Option<U256>,
    pub(crate) empty_scalars: bool,
}

impl L1BlockInfo {
    pub fn try_fetch<DB: Database>(db: &mut DB, spec_id: SpecId) -> Result<Self, DB::Error> {
        if spec_id.is_enabled_in(SpecId::CANCUN) {
            let _ = db.basic(L1_BLOCK_CONTRACT)?;
        }

        let l1_base_fee = db.storage(L1_BLOCK_CONTRACT, L1_BASE_FEE_SLOT)?;

        if !spec_id.is_enabled_in(SpecId::ECOTONE) {
            Ok(Self {
                l1_base_fee,
                l1_fee_overhead: Some(db.storage(L1_BLOCK_CONTRACT, L1_OVERHEAD_SLOT)?),
                l1_base_fee_scalar: db.storage(L1_BLOCK_CONTRACT, L1_SCALAR_SLOT)?,
                ..Default::default()
            })
        } else {
            let l1_blob_base_fee = db.storage(L1_BLOCK_CONTRACT, ECOTONE_L1_BLOB_BASE_FEE_SLOT)?;
            let l1_fee_scalars = db.storage(L1_BLOCK_CONTRACT, ECOTONE_L1_FEE_SCALARS_SLOT)?
                .to_be_bytes::<32>();

            let l1_base_fee_scalar = U256::from_be_slice(&l1_fee_scalars[BASE_FEE_SCALAR_OFFSET..BASE_FEE_SCALAR_OFFSET + 4]);
            let l1_blob_base_fee_scalar = U256::from_be_slice(&l1_fee_scalars[BLOB_BASE_FEE_SCALAR_OFFSET..BLOB_BASE_FEE_SCALAR_OFFSET + 4]);

            let empty_scalars = l1_blob_base_fee == U256::ZERO
                && l1_fee_scalars[BASE_FEE_SCALAR_OFFSET..BLOB_BASE_FEE_SCALAR_OFFSET + 4] == EMPTY_SCALARS;

            Ok(Self {
                l1_base_fee,
                l1_base_fee_scalar,
                l1_blob_base_fee: Some(l1_blob_base_fee),
                l1_blob_base_fee_scalar: Some(l1_blob_base_fee_scalar),
                empty_scalars,
                l1_fee_overhead: empty_scalars.then(|| db.storage(L1_BLOCK_CONTRACT, L1_OVERHEAD_SLOT)).transpose()?,
            })
        }
    }

    pub fn data_gas(&self, input: &[u8], spec_id: SpecId) -> U256 {
        let mut cost = U256::from(input.iter().fold(0, |acc, &byte| {
            acc + if byte == 0 { ZERO_BYTE_COST } else { NON_ZERO_BYTE_COST }
        }));

        if !spec_id.is_enabled_in(SpecId::REGOLITH) {
            cost += U256::from(NON_ZERO_BYTE_COST * 68);
        }

        cost
    }

    pub fn calculate_tx_l1_cost(&self, input: &[u8], spec_id: SpecId) -> U256 {
        if input.is_empty() || input.first() == Some(&0x7F) {
            return U256::ZERO;
        }

        if spec_id.is_enabled_in(SpecId::ECOTONE) && !self.empty_scalars {
            self.calculate_tx_l1_cost_ecotone(input, spec_id)
        } else {
            self.calculate_tx_l1_cost_bedrock(input, spec_id)
        }
    }

    fn calculate_tx_l1_cost_bedrock(&self, input: &[u8], spec_id: SpecId) -> U256 {
        self.data_gas(input, spec_id)
            .saturating_add(self.l1_fee_overhead.unwrap_or_default())
            .saturating_mul(self.l1_base_fee)
            .saturating_mul(self.l1_base_fee_scalar)
            .wrapping_div(U256::from(1_000_000))
    }

    fn calculate_tx_l1_cost_ecotone(&self, input: &[u8], spec_id: SpecId) -> U256 {
        let rollup_data_gas_cost = self.data_gas(input, spec_id);
        let calldata_cost_per_byte = self.l1_base_fee
            .saturating_mul(U256::from(16))
            .saturating_mul(self.l1_base_fee_scalar);
        let blob_cost_per_byte = self.l1_blob_base_fee
            .unwrap_or_default()
            .saturating_mul(self.l1_blob_base_fee_scalar.unwrap_or_default());

        calldata_cost_per_byte
            .saturating_add(blob_cost_per_byte)
            .saturating_mul(rollup_data_gas_cost)
            .wrapping_div(U256::from(1_000_000 * 16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::bytes;

    #[test]
    fn test_data_gas() {
        let l1_block_info = L1BlockInfo::default();

        let input = bytes!("FACADE");
        assert_eq!(l1_block_info.data_gas(&input, SpecId::BEDROCK), U256::from(1136));
        assert_eq!(l1_block_info.data_gas(&input, SpecId::REGOLITH), U256::from(48));

        let input = bytes!("FA00CA00DE");
        assert_eq!(l1_block_info.data_gas(&input, SpecId::BEDROCK), U256::from(1144));
        assert_eq!(l1_block_info.data_gas(&input, SpecId::REGOLITH), U256::from(56));
    }

    #[test]
    fn test_calculate_tx_l1_cost() {
        let l1_block_info = L1BlockInfo {
            l1_base_fee: U256::from(1_000),
            l1_fee_overhead: Some(U256::from(1_000)),
            l1_base_fee_scalar: U256::from(1_000),
            ..Default::default()
        };

        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!("FACADE"), SpecId::REGOLITH), U256::from(1048));
        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!(""), SpecId::REGOLITH), U256::ZERO);
        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!("7FFACADE"), SpecId::REGOLITH), U256::ZERO);
    }

    #[test]
    fn test_calculate_tx_l1_cost_ecotone() {
        let mut l1_block_info = L1BlockInfo {
            l1_base_fee: U256::from(1_000),
            l1_base_fee_scalar: U256::from(1_000),
            l1_blob_base_fee: Some(U256::from(1_000)),
            l1_blob_base_fee_scalar: Some(U256::from(1_000)),
            l1_fee_overhead: Some(U256::from(1_000)),
            ..Default::default()
        };

        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!("FACADE"), SpecId::ECOTONE), U256::from(51));
        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!(""), SpecId::ECOTONE), U256::ZERO);
        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!("7FFACADE"), SpecId::ECOTONE), U256::ZERO);

        l1_block_info.empty_scalars = true;
        assert_eq!(l1_block_info.calculate_tx_l1_cost(&bytes!("FACADE"), SpecId::ECOTONE), U256::from(1048));
    }
}
