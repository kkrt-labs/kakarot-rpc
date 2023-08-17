pub mod balance;
pub mod block;
pub mod call;
pub mod convertible;
pub mod event;
pub mod event_filter;
pub mod felt;
pub mod signature;
#[cfg(test)]
pub mod tests;
pub mod transaction;
pub mod transaction_receipt;

use ruint::FromUintError;
use starknet::core::types::FromByteArrayError;
use thiserror::Error;

use crate::client::helpers::DataDecodingError;

#[derive(Debug, Error)]
/// Conversion error
pub enum ConversionError<T> {
    /// Ethereum to Starknet transaction conversion error
    #[error("transaction conversion error: {0}")]
    TransactionConversionError(String),
    /// Felt252Wrapper conversion error
    #[error(transparent)]
    Felt252WrapperConversionError(#[from] FromByteArrayError),
    /// Data decoding error
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
    /// Felt252Wrapper to Ethereum address conversion error
    #[error(
        "failed to convert Felt252Wrapper to Ethereum address: the value exceeds the maximum size of an Ethereum \
         address"
    )]
    ToEthereumAddressError,
    /// Value out of range error
    #[error("value out of range: {0}")]
    ValueOutOfRange(String),
    /// Uint conversion error
    #[error(transparent)]
    UintConversionError(#[from] FromUintError<T>),
    /// Other conversion error
    #[error("failed to convert value: {0}")]
    Other(String),
}
