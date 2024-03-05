use jsonrpsee::types::ErrorObject;
use thiserror::Error;

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

/// Error that can occur when interacting with the provider.
#[derive(Debug, Error)]
pub enum EthProviderError {
    /// MongoDB error.
    #[error(transparent)]
    MongoDbError(#[from] mongodb::error::Error),
    /// Starknet Provider error.
    #[error(transparent)]
    StarknetProviderError(#[from] starknet::providers::ProviderError),
    /// EVM execution error.
    #[error("EVM execution error: {0}")]
    EvmExecutionError(String),
    /// Contract call error.
    #[error(transparent)]
    ContractCallError(#[from] cainome::cairo_serde::Error),
    /// Conversion error.
    #[error(transparent)]
    ConversionError(#[from] crate::models::errors::ConversionError),
    /// Value not found in the database.
    #[error("{0} not found.")]
    ValueNotFound(String),
    /// Method not supported.
    #[error("Method not supported: {0}")]
    MethodNotSupported(String),
    /// Other error.
    #[error(transparent)]
    Other(#[from] eyre::Error),
}

impl From<EthProviderError> for ErrorObject<'static> {
    fn from(value: EthProviderError) -> Self {
        let msg = value.to_string();
        match value {
            EthProviderError::MongoDbError(msg) => rpc_err(EthRpcErrorCode::ResourceNotFound, msg.to_string()),
            EthProviderError::StarknetProviderError(msg) => rpc_err(EthRpcErrorCode::InternalError, msg.to_string()),
            EthProviderError::EvmExecutionError(_) => rpc_err(EthRpcErrorCode::ExecutionError, msg),
            EthProviderError::ContractCallError(msg) => rpc_err(EthRpcErrorCode::ExecutionError, msg.to_string()),
            EthProviderError::ConversionError(msg) => rpc_err(EthRpcErrorCode::ParseError, msg.to_string()),
            EthProviderError::ValueNotFound(_) => rpc_err(EthRpcErrorCode::ResourceNotFound, msg),
            EthProviderError::MethodNotSupported(_) => rpc_err(EthRpcErrorCode::MethodNotSupported, msg),
            EthProviderError::Other(msg) => rpc_err(EthRpcErrorCode::InternalError, msg.to_string()),
        }
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
pub fn rpc_err(code: EthRpcErrorCode, msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(code as i32, msg.into(), None::<()>)
}
