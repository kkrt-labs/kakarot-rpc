use jsonrpsee::types::ErrorObject;
use starknet::providers::ProviderError as StarknetProviderError;
use thiserror::Error;

use crate::starknet_client::errors::{rpc_err, EthRpcErrorCode};

/// Error that can occur when interacting with the database.
#[derive(Debug, Error)]
pub enum EthProviderError {
    /// MongoDB error.
    #[error(transparent)]
    MongoDbError(#[from] mongodb::error::Error),
    /// Starknet Provider error.
    #[error(transparent)]
    StarknetProviderError(#[from] StarknetProviderError),
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
