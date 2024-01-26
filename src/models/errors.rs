use jsonrpsee::types::ErrorObject;
use ruint::FromUintError;
use starknet::core::types::FromByteArrayError;
use thiserror::Error;

use crate::starknet_client::{errors::EthApiError, helpers::DataDecodingError};

#[derive(Debug, Error)]
/// Conversion error
pub enum ConversionError {
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
    #[error("Failed to convert Ethereum transaction to Starknet transaction: {0}")]
    ToStarknetTransactionError(String),
    /// Value out of range error
    #[error("value out of range: {0}")]
    ValueOutOfRange(String),
    /// Uint conversion error
    #[error("Uint conversion error: {0}")]
    UintConversionError(String),
    /// Other conversion error
    #[error("failed to convert value: {0}")]
    Other(String),
}

impl<T> From<FromUintError<T>> for ConversionError {
    fn from(err: FromUintError<T>) -> Self {
        Self::UintConversionError(err.to_string())
    }
}

impl From<ConversionError> for ErrorObject<'static> {
    fn from(err: ConversionError) -> Self {
        let err = EthApiError::from(err);
        err.into()
    }
}
