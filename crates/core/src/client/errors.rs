use jsonrpsee::types::error::{INTERNAL_ERROR_CODE, INVALID_PARAMS_CODE, SERVER_IS_BUSY_CODE, UNKNOWN_ERROR_CODE};
use jsonrpsee::types::ErrorObject;
use starknet::core::types::StarknetError;
use starknet::providers::jsonrpc::JsonRpcClientError;
use starknet::providers::ProviderError;
use thiserror::Error;

use super::helpers::DataDecodingError;
use crate::models::ConversionError;

/// List of JSON-RPC error codes from reth
#[derive(Debug, Copy, PartialEq, Eq, Clone)]
pub enum EthRpcErrorCode {
    /// Failed to send transaction, See also <https://github.com/MetaMask/eth-rpc-errors/blob/main/src/error-constants.ts>
    TransactionRejected = -32003,
    /// Custom geth error code, <https://github.com/vapory-legacy/wiki/blob/master/JSON-RPC-Error-Codes-Improvement-Proposal.md>
    ExecutionError = 3,
    /// <https://eips.ethereum.org/EIPS/eip-1898>
    InvalidInput = -32000,
    /// Thrown when a block wasn't found <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1898.md>
    /// > If the block is not found, the callee SHOULD raise a JSON-RPC error (the recommended
    /// > error code is -32001: Resource not found).
    ResourceNotFound = -32001,
}

/// Error that can accure when interacting with the Kakarot ETH API.
#[derive(Debug, Error)]
pub enum EthApiError {
    /// Request to the Starknet provider failed.
    #[error(transparent)]
    RequestError(#[from] ProviderError<JsonRpcClientError<reqwest::Error>>),
    /// Conversion between Starknet types and ETH failed.
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
    /// Data decoding into ETH types failed.
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
    /// Other error.
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

impl From<EthApiError> for ErrorObject<'static> {
    fn from(error: EthApiError) -> Self {
        match error {
            EthApiError::RequestError(err_provider) => match err_provider {
                ProviderError::StarknetError(err) => match err {
                    StarknetError::BlockNotFound
                    | StarknetError::ClassHashNotFound
                    | StarknetError::ContractNotFound
                    | StarknetError::NoBlocks
                    | StarknetError::TransactionHashNotFound => {
                        rpc_err(EthRpcErrorCode::ResourceNotFound as i32, err_provider.to_string())
                    }
                    StarknetError::ContractError => {
                        rpc_err(EthRpcErrorCode::ExecutionError as i32, err_provider.to_string())
                    }
                    StarknetError::InvalidContractClass
                    | StarknetError::InvalidContinuationToken
                    | StarknetError::InvalidTransactionIndex
                    | StarknetError::PageSizeTooBig
                    | StarknetError::TooManyKeysInFilter
                    | StarknetError::ClassAlreadyDeclared => {
                        rpc_err(EthRpcErrorCode::InvalidInput as i32, err_provider.to_string())
                    }
                    StarknetError::FailedToReceiveTransaction => {
                        rpc_err(EthRpcErrorCode::TransactionRejected as i32, err_provider.to_string())
                    }
                },
                ProviderError::ArrayLengthMismatch => rpc_err(INVALID_PARAMS_CODE, err_provider.to_string()),
                ProviderError::RateLimited => rpc_err(SERVER_IS_BUSY_CODE, err_provider.to_string()),
                ProviderError::Other(_) => rpc_err(UNKNOWN_ERROR_CODE, err_provider.to_string()),
            },
            EthApiError::ConversionError(err) => rpc_err(INTERNAL_ERROR_CODE, err.to_string()),
            EthApiError::DataDecodingError(err) => rpc_err(INTERNAL_ERROR_CODE, err.to_string()),
            EthApiError::OtherError(err) => rpc_err(INTERNAL_ERROR_CODE, err.to_string()),
        }
    }
}

impl From<EthApiError> for jsonrpsee::core::Error {
    fn from(err: EthApiError) -> Self {
        jsonrpsee::core::Error::Call(err.into())
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
pub fn rpc_err(code: i32, msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(code, msg.into(), None::<()>)
}
