use ruint::FromUintError;
use starknet::core::types::FromByteArrayError;
use thiserror::Error;

#[derive(Debug, Error)]
/// Conversion error
pub enum ConversionError {
    /// Felt252Wrapper conversion error
    #[error(transparent)]
    Felt252WrapperConversionError(#[from] FromByteArrayError),
    /// Felt252Wrapper to Ethereum address conversion error
    #[error(
        "failed to convert Felt252Wrapper to Ethereum address: the value exceeds the maximum size of an Ethereum \
         address"
    )]
    ToEthereumAddressError,
    #[error("Failed to convert Ethereum transaction to Starknet transaction: {0}")]
    ToStarknetTransactionError(String),
    /// Value out of range error
    #[error("value out of range: {0}")]
    ValueOutOfRange(String),
    /// Uint conversion error
    #[error("Uint conversion error: {0}")]
    UintConversionError(String),
    /// Transaction conversion error
    #[error("Transaction conversion error: {0}")]
    TransactionConversionError(String),
    /// Other conversion error
    #[error("failed to convert value: {0}")]
    Other(String),
}

impl<T> From<FromUintError<T>> for ConversionError {
    fn from(err: FromUintError<T>) -> Self {
        Self::UintConversionError(err.to_string())
    }
}
