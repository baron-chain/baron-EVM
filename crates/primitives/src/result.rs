use crate::{Address, Bytes, Log, State, U256};
use core::fmt;
use std::{boxed::Box, string::String, vec::Vec};

pub type EVMResult<DBError> = EVMResultGeneric<ResultAndState, DBError>;
pub type EVMResultGeneric<T, DBError> = core::result::Result<T, EVMError<DBError>>;

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResultAndState {
    pub result: ExecutionResult,
    pub state: State,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum ExecutionResult {
    Success { reason: SuccessReason, gas_used: u64, gas_refunded: u64, logs: Vec<Log>, output: Output },
    Revert { gas_used: u64, output: Bytes },
    Halt { reason: HaltReason, gas_used: u64 },
}

impl ExecutionResult {
    pub fn is_success(&self) -> bool { matches!(self, Self::Success { .. }) }
    pub fn is_halt(&self) -> bool { matches!(self, Self::Halt { .. }) }
    pub fn output(&self) -> Option<&Bytes> {
        match self {
            Self::Success { output, .. } => Some(output.data()),
            Self::Revert { output, .. } => Some(output),
            _ => None,
        }
    }
    pub fn into_output(self) -> Option<Bytes> {
        match self {
            Self::Success { output, .. } => Some(output.into_data()),
            Self::Revert { output, .. } => Some(output),
            _ => None,
        }
    }
    pub fn logs(&self) -> &[Log] {
        match self {
            Self::Success { logs, .. } => logs,
            _ => &[],
        }
    }
    pub fn into_logs(self) -> Vec<Log> {
        match self {
            Self::Success { logs, .. } => logs,
            _ => Vec::new(),
        }
    }
    pub fn gas_used(&self) -> u64 {
        match *self {
            Self::Success { gas_used, .. } | Self::Revert { gas_used, .. } | Self::Halt { gas_used, .. } => gas_used,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Output {
    Call(Bytes),
    Create(Bytes, Option<Address>),
}

impl Output {
    pub fn into_data(self) -> Bytes {
        match self {
            Output::Call(data) | Output::Create(data, _) => data,
        }
    }
    pub fn data(&self) -> &Bytes {
        match self {
            Output::Call(data) | Output::Create(data, _) => data,
        }
    }
    pub fn address(&self) -> Option<&Address> {
        match self {
            Output::Call(_) => None,
            Output::Create(_, address) => address.as_ref(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum EVMError<DBError> {
    Transaction(InvalidTransaction),
    Header(InvalidHeader),
    Database(DBError),
    Custom(String),
}

#[cfg(feature = "std")]
impl<DBError: std::error::Error + 'static> std::error::Error for EVMError<DBError> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transaction(e) => Some(e),
            Self::Header(e) => Some(e),
            Self::Database(e) => Some(e),
            Self::Custom(_) => None,
        }
    }
}

impl<DBError: fmt::Display> fmt::Display for EVMError<DBError> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transaction(e) => write!(f, "transaction validation error: {e}"),
            Self::Header(e) => write!(f, "header validation error: {e}"),
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::Custom(e) => f.write_str(e),
        }
    }
}

impl<DBError> From<InvalidTransaction> for EVMError<DBError> {
    fn from(value: InvalidTransaction) -> Self { Self::Transaction(value) }
}

impl<DBError> From<InvalidHeader> for EVMError<DBError> {
    fn from(value: InvalidHeader) -> Self { Self::Header(value) }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvalidTransaction {
    PriorityFeeGreaterThanMaxFee,
    GasPriceLessThanBasefee,
    CallerGasLimitMoreThanBlock,
    CallGasCostMoreThanGasLimit,
    RejectCallerWithCode,
    LackOfFundForMaxFee { fee: Box<U256>, balance: Box<U256> },
    OverflowPaymentInTransaction,
    NonceOverflowInTransaction,
    NonceTooHigh { tx: u64, state: u64 },
    NonceTooLow { tx: u64, state: u64 },
    CreateInitCodeSizeLimit,
    InvalidChainId,
    AccessListNotSupported,
    MaxFeePerBlobGasNotSupported,
    BlobVersionedHashesNotSupported,
    BlobGasPriceGreaterThanMax,
    EmptyBlobs,
    BlobCreateTransaction,
    TooManyBlobs,
    BlobVersionNotSupported,
    EofInitcodesNotSupported,
    EofInitcodesNumberLimit,
    EofInitcodesSizeLimit,
    EofCrateShouldHaveToAddress,
    #[cfg(feature = "optimism")]
    DepositSystemTxPostRegolith,
    #[cfg(feature = "optimism")]
    HaltedDepositPostRegolith,
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidTransaction {}

impl fmt::Display for InvalidTransaction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PriorityFeeGreaterThanMaxFee => write!(f, "priority fee is greater than max fee"),
            Self::GasPriceLessThanBasefee => write!(f, "gas price is less than basefee"),
            Self::CallerGasLimitMoreThanBlock => write!(f, "caller gas limit exceeds the block gas limit"),
            Self::CallGasCostMoreThanGasLimit => write!(f, "call gas cost exceeds the gas limit"),
            Self::RejectCallerWithCode => write!(f, "reject transactions from senders with deployed code"),
            Self::LackOfFundForMaxFee { fee, balance } => write!(f, "lack of funds ({balance}) for max fee ({fee})"),
            Self::OverflowPaymentInTransaction => write!(f, "overflow payment in transaction"),
            Self::NonceOverflowInTransaction => write!(f, "nonce overflow in transaction"),
            Self::NonceTooHigh { tx, state } => write!(f, "nonce {tx} too high, expected {state}"),
            Self::NonceTooLow { tx, state } => write!(f, "nonce {tx} too low, expected {state}"),
            Self::CreateInitCodeSizeLimit => write!(f, "create initcode size limit"),
            Self::InvalidChainId => write!(f, "invalid chain ID"),
            Self::AccessListNotSupported => write!(f, "access list not supported"),
            Self::MaxFeePerBlobGasNotSupported => write!(f, "max fee per blob gas not supported"),
            Self::BlobVersionedHashesNotSupported => write!(f, "blob versioned hashes not supported"),
            Self::BlobGasPriceGreaterThanMax => write!(f, "blob gas price is greater than max fee per blob gas"),
            Self::EmptyBlobs => write!(f, "empty blobs"),
            Self::BlobCreateTransaction => write!(f, "blob create transaction"),
            Self::TooManyBlobs => write!(f, "too many blobs"),
            Self::BlobVersionNotSupported => write!(f, "blob version not supported"),
            Self::EofInitcodesNotSupported => write!(f, "EOF initcodes not supported"),
            Self::EofCrateShouldHaveToAddress => write!(f, "EOF crate should have `to` address"),
            Self::EofInitcodesSizeLimit => write!(f, "EOF initcodes size limit"),
            Self::EofInitcodesNumberLimit => write!(f, "EOF initcodes number limit"),
            #[cfg(feature = "optimism")]
            Self::DepositSystemTxPostRegolith => write!(f, "deposit system transactions post regolith hardfork are not supported"),
            #[cfg(feature = "optimism")]
            Self::HaltedDepositPostRegolith => write!(f, "deposit transaction halted post-regolith; error will be bubbled up to main return handler"),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum InvalidHeader {
    PrevrandaoNotSet,
    ExcessBlobGasNotSet,
}

#[cfg(feature = "std")]
impl std::error::Error for InvalidHeader {}

impl fmt::Display for InvalidHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PrevrandaoNotSet => write!(f, "`prevrandao` not set"),
            Self::ExcessBlobGasNotSet => write!(f, "`excess_blob_gas` not set"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SuccessReason { Stop, Return, SelfDestruct }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum HaltReason {
    OutOfGas(OutOfGasError), OpcodeNotFound, InvalidFEOpcode, InvalidJump, NotActivated,
    StackUnderflow, StackOverflow, OutOfOffset, CreateCollision, PrecompileError, NonceOverflow,
    CreateContractSizeLimit, CreateContractStartingWithEF, CreateInitCodeSizeLimit,
    OverflowPayment, StateChangeDuringStaticCall, CallNotAllowedInsideStatic, OutOfFunds, CallTooDeep,
    #[cfg(feature = "optimism")]
    FailedDeposit,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum OutOfGasError {
    Basic, MemoryLimit, Memory, Precompile, InvalidOperand,
}
