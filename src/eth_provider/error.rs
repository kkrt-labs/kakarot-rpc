use jsonrpsee::types::ErrorObject;
use starknet::providers::ProviderError as StarknetProviderError;
use thiserror::Error;

use crate::{
    models::errors::ConversionError,
    starknet_client::errors::{rpc_err, EthRpcErrorCode},
};

/// Error that can occur when interacting with the database.
#[derive(Debug, Error)]
pub enum EthProviderError {
    /// MongoDB error.
    #[error(transparent)]
    MongoDbError(#[from] mongodb::error::Error),
    /// Starknet Provider error.
    #[error(transparent)]
    StarknetProviderError(#[from] StarknetProviderError),
    /// EVM execution error.
    #[error("EVM execution error: {0}")]
    EvmExecutionError(String),
    /// Contract call error.
    #[error(transparent)]
    ContractCallError(#[from] starknet_abigen_parser::cairo_types::Error),
    /// Conversion error.
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    /// Value not found in the database.
    #[error("Did not find value in the database.")]
    ValueNotFound,
}

impl From<EthProviderError> for jsonrpsee::core::Error {
    fn from(err: EthProviderError) -> Self {
        Self::Call(err.into())
    }
}

impl From<EthProviderError> for ErrorObject<'static> {
    fn from(value: EthProviderError) -> Self {
        rpc_err(EthRpcErrorCode::InternalError, value.to_string())
    }
}
