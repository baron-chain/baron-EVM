pub mod handler_cfg;
pub use handler_cfg::{CfgEnvWithHandlerCfg, EnvWithHandlerCfg, HandlerCfg};

use crate::{
    calc_blob_gasprice, Account, Address, Bytes, HashMap, InvalidHeader, InvalidTransaction, Spec,
    SpecId, B256, GAS_PER_BLOB, KECCAK_EMPTY, MAX_BLOB_NUMBER_PER_BLOCK, MAX_INITCODE_SIZE, U256,
    VERSIONED_HASH_VERSION_KZG,
};
use core::cmp::{min, Ordering};
use core::hash::Hash;
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Env {
    pub cfg: CfgEnv,
    pub block: BlockEnv,
    pub tx: TxEnv,
}

impl Env {
    pub fn clear(&mut self) { *self = Self::default(); }
    pub fn boxed(cfg: CfgEnv, block: BlockEnv, tx: TxEnv) -> Box<Self> { Box::new(Self { cfg, block, tx }) }
    pub fn effective_gas_price(&self) -> U256 {
        self.tx.gas_priority_fee.map_or(self.tx.gas_price, |priority_fee| 
            min(self.tx.gas_price, self.block.basefee + priority_fee))
    }
    pub fn calc_data_fee(&self) -> Option<U256> {
        self.block.get_blob_gasprice().map(|price| U256::from(price).saturating_mul(U256::from(self.tx.get_total_blob_gas())))
    }
    pub fn calc_max_data_fee(&self) -> Option<U256> {
        self.tx.max_fee_per_blob_gas.map(|max_fee| max_fee.saturating_mul(U256::from(self.tx.get_total_blob_gas())))
    }
    pub fn validate_block_env<SPEC: Spec>(&self) -> Result<(), InvalidHeader> {
        if SPEC::enabled(SpecId::MERGE) && self.block.prevrandao.is_none() { return Err(InvalidHeader::PrevrandaoNotSet); }
        if SPEC::enabled(SpecId::CANCUN) && self.block.blob_excess_gas_and_price.is_none() { return Err(InvalidHeader::ExcessBlobGasNotSet); }
        Ok(())
    }
    pub fn validate_tx<SPEC: Spec>(&self) -> Result<(), InvalidTransaction> {
        if SPEC::enabled(SpecId::LONDON) {
            if let Some(priority_fee) = self.tx.gas_priority_fee {
                if priority_fee > self.tx.gas_price { return Err(InvalidTransaction::PriorityFeeGreaterThanMaxFee); }
            }
            if !self.cfg.is_base_fee_check_disabled() && self.effective_gas_price() < self.block.basefee {
                return Err(InvalidTransaction::GasPriceLessThanBasefee);
            }
        }
        if !self.cfg.is_block_gas_limit_disabled() && U256::from(self.tx.gas_limit) > self.block.gas_limit {
            return Err(InvalidTransaction::CallerGasLimitMoreThanBlock);
        }
        if SPEC::enabled(SpecId::SHANGHAI) && self.tx.transact_to.is_create() {
            let max_initcode_size = self.cfg.limit_contract_code_size.map_or(MAX_INITCODE_SIZE, |limit| limit.saturating_mul(2));
            if self.tx.data.len() > max_initcode_size { return Err(InvalidTransaction::CreateInitCodeSizeLimit); }
        }
        if let Some(tx_chain_id) = self.tx.chain_id {
            if tx_chain_id != self.cfg.chain_id { return Err(InvalidTransaction::InvalidChainId); }
        }
        if !SPEC::enabled(SpecId::BERLIN) && !self.tx.access_list.is_empty() { return Err(InvalidTransaction::AccessListNotSupported); }
        if SPEC::enabled(SpecId::CANCUN) {
            if let Some(max) = self.tx.max_fee_per_blob_gas {
                let price = self.block.get_blob_gasprice().expect("already checked");
                if U256::from(price) > max { return Err(InvalidTransaction::BlobGasPriceGreaterThanMax); }
                if self.tx.blob_hashes.is_empty() { return Err(InvalidTransaction::EmptyBlobs); }
                if self.tx.transact_to.is_create() { return Err(InvalidTransaction::BlobCreateTransaction); }
                if self.tx.blob_hashes.iter().any(|blob| blob[0] != VERSIONED_HASH_VERSION_KZG) {
                    return Err(InvalidTransaction::BlobVersionNotSupported);
                }
                if self.tx.blob_hashes.len() > MAX_BLOB_NUMBER_PER_BLOCK as usize { return Err(InvalidTransaction::TooManyBlobs); }
            }
        } else {
            if !self.tx.blob_hashes.is_empty() { return Err(InvalidTransaction::BlobVersionedHashesNotSupported); }
            if self.tx.max_fee_per_blob_gas.is_some() { return Err(InvalidTransaction::MaxFeePerBlobGasNotSupported); }
        }
        if SPEC::enabled(SpecId::PRAGUE) {
            if !self.tx.eof_initcodes.is_empty() {
                if !self.tx.blob_hashes.is_empty() { return Err(InvalidTransaction::BlobVersionedHashesNotSupported); }
                if self.tx.max_fee_per_blob_gas.is_some() { return Err(InvalidTransaction::MaxFeePerBlobGasNotSupported); }
                if matches!(self.tx.transact_to, TransactTo::Call(_)) { return Err(InvalidTransaction::EofCrateShouldHaveToAddress); }
            } else {
                if self.tx.eof_initcodes.len() > 256 { return Err(InvalidTransaction::EofInitcodesNumberLimit); }
                if self.tx.eof_initcodes_hashed.iter().any(|(_, i)| i.len() >= MAX_INITCODE_SIZE) {
                    return Err(InvalidTransaction::EofInitcodesSizeLimit);
                }
            }
        } else if !self.tx.eof_initcodes.is_empty() { return Err(InvalidTransaction::EofInitcodesNotSupported); }
        Ok(())
    }
    pub fn validate_tx_against_state<SPEC: Spec>(&self, account: &mut Account) -> Result<(), InvalidTransaction> {
        if !self.cfg.is_eip3607_disabled() && account.info.code_hash != KECCAK_EMPTY {
            return Err(InvalidTransaction::RejectCallerWithCode);
        }
        if let Some(tx) = self.tx.nonce {
            let state = account.info.nonce;
            match tx.cmp(&state) {
                Ordering::Greater => return Err(InvalidTransaction::NonceTooHigh { tx, state }),
                Ordering::Less => return Err(InvalidTransaction::NonceTooLow { tx, state }),
                _ => {}
            }
        }
        let mut balance_check = U256::from(self.tx.gas_limit)
            .checked_mul(self.tx.gas_price)
            .and_then(|gas_cost| gas_cost.checked_add(self.tx.value))
            .ok_or(InvalidTransaction::OverflowPaymentInTransaction)?;
        if SPEC::enabled(SpecId::CANCUN) {
            balance_check = balance_check
                .checked_add(self.calc_max_data_fee().unwrap_or_default())
                .ok_or(InvalidTransaction::OverflowPaymentInTransaction)?;
        }
        if balance_check > account.info.balance {
            if self.cfg.is_balance_check_disabled() {
                account.info.balance = balance_check;
            } else {
                return Err(InvalidTransaction::LackOfFundForMaxFee {
                    fee: Box::new(balance_check),
                    balance: Box::new(account.info.balance),
                });
            }
        }
        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct CfgEnv {
    pub chain_id: u64,
    #[cfg(feature = "c-kzg")]
    #[cfg_attr(feature = "serde", serde(skip))]
    pub kzg_settings: crate::kzg::EnvKzgSettings,
    pub perf_analyse_created_bytecodes: AnalysisKind,
    pub limit_contract_code_size: Option<usize>,
    #[cfg(feature = "memory_limit")]
    pub memory_limit: u64,
    #[cfg(feature = "optional_balance_check")]
    pub disable_balance_check: bool,
    #[cfg(feature = "optional_block_gas_limit")]
    pub disable_block_gas_limit: bool,
    #[cfg(feature = "optional_eip3607")]
    pub disable_eip3607: bool,
    #[cfg(feature = "optional_gas_refund")]
    pub disable_gas_refund: bool,
    #[cfg(feature = "optional_no_base_fee")]
    pub disable_base_fee: bool,
    #[cfg(feature = "optional_beneficiary_reward")]
    pub disable_beneficiary_reward: bool,
}

impl CfgEnv {
    pub fn with_chain_id(mut self, chain_id: u64) -> Self { self.chain_id = chain_id; self }
    #[cfg(feature = "optional_eip3607")]
    pub fn is_eip3607_disabled(&self) -> bool { self.disable_eip3607 }
    #[cfg(not(feature = "optional_eip3607"))]
    pub fn is_eip3607_disabled(&self) -> bool { false }
    #[cfg(feature = "optional_balance_check")]
    pub fn is_balance_check_disabled(&self) -> bool { self.disable_balance_check }
    #[cfg(not(feature = "optional_balance_check"))]
    pub fn is_balance_check_disabled(&self) -> bool { false }
    #[cfg(feature = "optional_gas_refund")]
    pub fn is_gas_refund_disabled(&self) -> bool { self.disable_gas_refund }
    #[cfg(not(feature = "optional_gas_refund"))]
    pub fn is_gas_refund_disabled(&self) -> bool { false }
    #[cfg(feature = "optional_no_base_fee")]
    pub fn is_base_fee_check_disabled(&self) -> bool { self.disable_base_fee }
    #[cfg(not(feature = "optional_no_base_fee"))]
    pub fn is_base_fee_check_disabled(&self) -> bool { false }
    #[cfg(feature = "optional_block_gas_limit")]
    pub fn is_block_gas_limit_disabled(&self) -> bool { self.disable_block_gas_limit }
    #[cfg(not(feature = "optional_block_gas_limit"))]
    pub fn is_block_gas_limit_disabled(&self) -> bool { false }
    #[cfg(feature = "optional_beneficiary_reward")]
    pub fn is_beneficiary_reward_disabled(&self) -> bool { self.disable_beneficiary_reward }
    #[cfg(not(feature = "optional_beneficiary_reward"))]
    pub fn is_beneficiary_reward_disabled(&self) -> bool { false }
}

impl Default for CfgEnv {
    fn default() -> Self {
        Self {
            chain_id: 1,
            perf_analyse_created_bytecodes: AnalysisKind::default(),
            limit_contract_code_size: None,
            #[cfg(feature = "c-kzg")]
            kzg_settings: crate::kzg::EnvKzgSettings::Default,
            #[cfg(feature = "memory_limit")]
            memory_limit: (1 << 32) - 1,
            #[cfg(feature = "optional_balance_check")]
            disable_balance_check: false,
            #[cfg(feature = "optional_block_gas_limit")]
            disable_block_gas_limit: false,
            #[cfg(feature = "optional_eip3607")]
            disable_eip3607: false,
            #[cfg(feature = "optional_gas_refund")]
            disable_gas_refund: false,
            #[cfg(feature = "optional_no_base_fee")]
            disable_base_fee: false,
            #[cfg(feature = "optional_beneficiary_reward")]
            disable_beneficiary_reward: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlockEnv {
    pub number: U256,
    pub coinbase: Address,
    pub timestamp: U256,
    pub gas_limit: U256,
    pub basefee: U256,
    pub difficulty: U256,
    pub prevrandao: Option<B256>,
    pub blob_excess_gas_and_price: Option<BlobExcessGasAndPrice>,
}

impl BlockEnv {
    pub fn set_blob_excess_gas_and_price(&mut self, excess_blob_gas: u64) {
        self.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice::new(excess_blob_gas));
    }
    pub fn get_blob_gasprice(&self) -> Option<u128> {
        self.blob_excess_gas_and_price.as_ref().map(|a| a.blob_gasprice)
    }
    pub fn get_blob_excess_gas(&self) -> Option<u64> {
        self.blob_excess_gas_and_price.as_ref().map(|a| a.excess_blob_gas)
    }
    pub fn clear(&mut self) { *self = Self::default(); }
}

impl Default for BlockEnv {
    fn default() -> Self {
        Self {
            number: U256::ZERO,
            coinbase: Address::ZERO,
            timestamp: U256::from(1),
            gas_limit: U256::MAX,
            basefee: U256::ZERO,
            difficulty: U256::ZERO,
            prevrandao: Some(B256::ZERO),
            blob_excess_gas_and_price: Some(BlobExcessGasAndPrice::new(0)),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TxEnv {
    pub caller: Address,
    pub gas_limit: u64,
    pub gas_price: U256,
    pub transact_to: TransactTo,
    pub value: U256,
    pub data: Bytes,
    pub nonce: Option<u64>,
    pub chain_id: Option<u64>,
    pub access_list: Vec<(Address, Vec<U256>)>,
    pub gas_priority_fee: Option<U256>,
    pub blob_hashes: Vec<B256>,
    pub max_fee_per_blob_gas: Option<U256>,
    pub eof_initcodes: Vec<Bytes>,
    pub eof_initcodes_hashed: HashMap<B256, Bytes>,
    #[cfg_attr(feature = "serde", serde(flatten))]
    #[cfg(feature = "optimism")]
    pub optimism: OptimismFields,
}

impl TxEnv {
    pub fn get_total_blob_gas(&self) -> u64 { GAS_PER_BLOB * self.blob_hashes.len() as u64 }
    pub fn clear(&mut self) { *self = Self::default(); }
}

impl Default for TxEnv {
    fn default() -> Self {
        Self {
            caller: Address::ZERO,
            gas_limit: u64::MAX,
            gas_price: U256::ZERO,
            gas_priority_fee: None,
            transact_to: TransactTo::Call(Address::ZERO),
            value: U256::ZERO,
            data: Bytes::new(),
            chain_id: None,
            nonce: None,
            access_list: Vec::new(),
            blob_hashes: Vec::new(),
            max_fee_per_blob_gas: None,
            eof_initcodes: Vec::new(),
            eof_initcodes_hashed: HashMap::new(),
            #[cfg(feature = "optimism")]
            optimism: OptimismFields::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct BlobExcessGasAndPrice {
    pub excess_blob_gas: u64,
    pub blob_gasprice: u128,
}

impl BlobExcessGasAndPrice {
    pub fn new(excess_blob_gas: u64) -> Self {
        Self {
            excess_blob_gas,
            blob_gasprice: calc_blob_gasprice(excess_blob_gas),
        }
    }
}

#[cfg(feature = "optimism")]
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct OptimismFields {
    pub source_hash: Option<B256>,
    pub mint: Option<u128>,
    pub is_system_transaction: Option<bool>,
    pub enveloped_tx: Option<Bytes>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransactTo {
    Call(Address),
    Create,
}

impl TransactTo {
    pub fn call(address: Address) -> Self { Self::Call(address) }
    pub fn create() -> Self { Self::Create }
    pub fn is_call(&self) -> bool { matches!(self, Self::Call(_)) }
    pub fn is_create(&self) -> bool { matches!(self, Self::Create) }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum CreateScheme {
    Create,
    Create2 { salt: U256 },
}

#[derive(Clone, Default, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum AnalysisKind {
    Raw,
    #[default]
    Analyse,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tx_chain_id() {
        let mut env = Env::default();
        env.tx.chain_id = Some(1);
        env.cfg.chain_id = 2;
        assert_eq!(
            env.validate_tx::<crate::LatestSpec>(),
            Err(InvalidTransaction::InvalidChainId)
        );
    }

    #[test]
    fn test_validate_tx_access_list() {
        let mut env = Env::default();
        env.tx.access_list = vec![(Address::ZERO, vec![])];
        assert_eq!(
            env.validate_tx::<crate::FrontierSpec>(),
            Err(InvalidTransaction::AccessListNotSupported)
        );
    }
}
