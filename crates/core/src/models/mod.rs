pub mod balance;
pub mod block;
pub mod convertible;
pub mod event;
pub mod felt;
pub mod signature;
#[cfg(test)]
pub mod tests;
pub mod transaction;

use starknet::core::types::FromByteArrayError;
use thiserror::Error;

use crate::client::helpers::DataDecodingError;

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("transaction conversion error: {0}")]
    TransactionConversionError(String),
    #[error(transparent)]
    Felt252WrapperConversionError(#[from] FromByteArrayError),
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
    #[error(
        "Failed to convert Felt252Wrapper to Ethereum address: the value exceeds the maximum size of an Ethereum \
         address"
    )]
    ToEthereumAddressError,
}
