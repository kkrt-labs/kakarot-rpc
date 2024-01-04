use jsonrpsee::types::ErrorObject;
use starknet::core::types::{FromByteSliceError, StarknetError};
use starknet::providers::{MaybeUnknownErrorCode, ProviderError};
use thiserror::Error;

use super::helpers::DataDecodingError;
use crate::models::errors::ConversionError;

use starknet_abigen_parser::cairo_types::Error as AbigenError;

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

// Error that can accure when preparing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Missing mandatory environment variable error.
    #[error("Missing mandatory environment variable: {0}")]
    EnvironmentVariableMissing(String),
    /// Environment variable set wrong error.
    #[error("Environment variable {0} set wrong: {1}")]
    EnvironmentVariableSetWrong(String, String),
    /// Invalid URL error.
    #[error("Invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    /// Invalid network error.
    #[error("Invalid network: {0}")]
    InvalidNetwork(String),
}

/// Error that can accure when interacting with the Kakarot ETH API.
#[derive(Debug, Error)]
pub enum EthApiError {
    /// Request to the Starknet provider failed.
    #[error(transparent)]
    RequestError(#[from] ProviderError),
    /// Contract call with abigen failed.
    #[error(transparent)]
    AbigenError(#[from] AbigenError),
    /// Conversion between Starknet types and ETH failed.
    #[error("conversion error: {0}")]
    ConversionError(String),
    /// Data decoding into ETH types failed.
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
    /// Data not part of Kakarot.
    #[error("{0} not from Kakarot")]
    KakarotDataFilteringError(String),
    /// Feeder gateway error.
    #[error("Feeder gateway error: {0}")]
    FeederGatewayError(String),
    /// Missing parameter error.
    #[error("Missing parameter: {0}")]
    MissingParameterError(String),
    /// Configuration error.
    #[error(transparent)]
    ConfigError(#[from] ConfigError),
    /// Method not supported error.
    #[error("Method not supported: {0}")]
    MethodNotSupported(String),
    /// Execution Error
    #[error("Kakarot Execution Error, Reverted With: {0}")]
    EVMExecutionError(String),
    /// Other error.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<ConversionError> for EthApiError {
    fn from(err: ConversionError) -> Self {
        Self::ConversionError(err.to_string())
    }
}

impl From<FromByteSliceError> for EthApiError {
    fn from(err: FromByteSliceError) -> Self {
        Self::ConversionError(format!("Failed to convert from byte slice: {}", err))
    }
}

impl From<EthApiError> for ErrorObject<'static> {
    fn from(error: EthApiError) -> Self {
        match error {
            EthApiError::RequestError(err_provider) => match err_provider {
                ProviderError::StarknetError(err_with_msg) => match err_with_msg.code {
                    MaybeUnknownErrorCode::Known(err) => match err {
                        StarknetError::BlockNotFound
                        | StarknetError::ClassHashNotFound
                        | StarknetError::ContractNotFound
                        | StarknetError::NoBlocks
                        | StarknetError::TransactionHashNotFound
                        | StarknetError::InvalidBlockHash
                        | StarknetError::InvalidTransactionHash
                        | StarknetError::NoTraceAvailable => {
                            rpc_err(EthRpcErrorCode::ResourceNotFound, format!("{err}: {}", err_with_msg.message))
                        }
                        StarknetError::ContractError => {
                            rpc_err(EthRpcErrorCode::ExecutionError, format!("{err}: {}", err_with_msg.message))
                        }
                        StarknetError::InvalidTransactionNonce
                        | StarknetError::InvalidContinuationToken
                        | StarknetError::InvalidTransactionIndex
                        | StarknetError::PageSizeTooBig
                        | StarknetError::TooManyKeysInFilter
                        | StarknetError::InsufficientAccountBalance
                        | StarknetError::InsufficientMaxFee
                        | StarknetError::ClassAlreadyDeclared
                        | StarknetError::UnsupportedTxVersion
                        | StarknetError::CompilationFailed => {
                            rpc_err(EthRpcErrorCode::InvalidInput, format!("{err}: {}", err_with_msg.message))
                        }
                        StarknetError::FailedToReceiveTransaction
                        | StarknetError::DuplicateTx
                        | StarknetError::NonAccount
                        | StarknetError::ValidationFailure
                        | StarknetError::UnsupportedContractClassVersion
                        | StarknetError::ContractClassSizeIsTooLarge
                        | StarknetError::CompiledClassHashMismatch
                        | StarknetError::UnexpectedError => {
                            rpc_err(EthRpcErrorCode::TransactionRejected, format!("{err}: {}", err_with_msg.message))
                        }
                    },
                    MaybeUnknownErrorCode::Unknown(code) => {
                        rpc_err(EthRpcErrorCode::Unknown, format!("got code {} with: {}", code, err_with_msg.message))
                    }
                },
                ProviderError::ArrayLengthMismatch => rpc_err(EthRpcErrorCode::InvalidParams, err_provider.to_string()),
                ProviderError::RateLimited => rpc_err(EthRpcErrorCode::RequestLimitExceeded, err_provider.to_string()),
                ProviderError::Other(_) => rpc_err(EthRpcErrorCode::InternalError, err_provider.to_string()),
            },
            EthApiError::AbigenError(err) => rpc_err(EthRpcErrorCode::InternalError, err.to_string()),
            EthApiError::ConversionError(err) => rpc_err(EthRpcErrorCode::InternalError, err),
            EthApiError::DataDecodingError(err) => rpc_err(EthRpcErrorCode::InternalError, err.to_string()),
            EthApiError::KakarotDataFilteringError(err) => rpc_err(EthRpcErrorCode::InternalError, err),
            EthApiError::FeederGatewayError(err) => rpc_err(EthRpcErrorCode::InternalError, err),
            EthApiError::MissingParameterError(err) => rpc_err(EthRpcErrorCode::InvalidParams, err),
            EthApiError::ConfigError(err) => rpc_err(EthRpcErrorCode::InternalError, err.to_string()),
            EthApiError::MethodNotSupported(err) => rpc_err(EthRpcErrorCode::MethodNotSupported, err),
            EthApiError::EVMExecutionError(err) => rpc_err(EthRpcErrorCode::ExecutionError, err),
            EthApiError::Other(err) => rpc_err(EthRpcErrorCode::InternalError, err.to_string()),
        }
    }
}

impl From<EthApiError> for jsonrpsee::core::Error {
    fn from(err: EthApiError) -> Self {
        Self::Call(err.into())
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
pub fn rpc_err(code: EthRpcErrorCode, msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(code as i32, msg.into(), None::<()>)
}
