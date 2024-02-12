use jsonrpsee::types::ErrorObject;
use starknet::providers::ProviderError as StarknetProviderError;
use thiserror::Error;

use crate::models::errors::ConversionError;

/// List of JSON-RPC error codes from ETH rpc spec.
/// https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1474.md
#[derive(Debug, Copy, PartialEq, Eq, Clone)]
pub enum EthRpcErrorCode {
    /// Custom geth error code, <https://github.com/vapory-legacy/wiki/blob/master/JSON-RPC-Error-Codes-Improvement-Proposal.md>
    Unknown,
    ExecutionError = 3,
    ParseError = -32700,
    InvalidRequest = -32600,
    MethodNotFound = -32601,
    InvalidParams = -32602,
    InternalError = -32603,
    InvalidInput = -32000,
    ResourceNotFound = -32001,
    ResourceUnavailable = -32002,
    TransactionRejected = -32003,
    MethodNotSupported = -32004,
    RequestLimitExceeded = -32005,
    JsonRpcVersionUnsupported = -32006,
}

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
    ContractCallError(#[from] cainome::cairo_serde::Error),
    /// Conversion error.
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    /// Value not found in the database.
    #[error("Did not find value in the database.")]
    ValueNotFound,
    /// Method not supported.
    #[error("Method not supported: {0}")]
    MethodNotSupported(String),
}

impl From<EthProviderError> for ErrorObject<'static> {
    fn from(value: EthProviderError) -> Self {
        rpc_err(EthRpcErrorCode::InternalError, value.to_string())
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
pub fn rpc_err(code: EthRpcErrorCode, msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(code as i32, msg.into(), None::<()>)
}
