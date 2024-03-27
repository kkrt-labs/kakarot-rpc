use jsonrpsee::types::ErrorObject;
use thiserror::Error;

use crate::models::ConversionError;

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

/// Error that can occur when interacting with the ETH Api.
#[derive(Debug, Error)]
pub enum EthApiError {
    /// When a block is not found
    #[error("unknown block")]
    UnknownBlock,
    /// When an unknown block number is encountered
    #[error("unknown block number")]
    UnknownBlockNumber,
    /// When an invalid block range is provided
    #[error("invalid block range")]
    InvalidBlockRange,
    /// Conversion error.
    #[error("transaction conversion error")]
    TransactionConversionError,
    /// Error related to receipt
    #[error("receipt error: {0}")]
    ReceiptError(#[from] ReceiptError),
    /// Error related to transaction
    #[error("transaction error: {0}")]
    TransactionError(#[from] TransactionError),
    /// Error related to signing
    #[error("signature error: {0}")]
    SignatureError(#[from] SignatureError),
    /// Unsupported feature
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// Other internal error
    #[error("internal error: {0}")]
    Internal(KakarotError),
}

impl From<EthApiError> for ErrorObject<'static> {
    fn from(value: EthApiError) -> Self {
        let msg = format!("{:?}", value);
        match value {
            EthApiError::UnknownBlock => rpc_err(EthRpcErrorCode::ResourceNotFound, msg),
            EthApiError::UnknownBlockNumber => rpc_err(EthRpcErrorCode::ResourceNotFound, msg),
            EthApiError::InvalidBlockRange => rpc_err(EthRpcErrorCode::InvalidParams, msg),
            EthApiError::TransactionConversionError => rpc_err(EthRpcErrorCode::InvalidParams, msg),
            EthApiError::TransactionError(err) => rpc_err(err.error_code(), msg),
            EthApiError::SignatureError(_) => rpc_err(EthRpcErrorCode::InvalidParams, msg),
            EthApiError::ReceiptError(_) => rpc_err(EthRpcErrorCode::InvalidParams, msg),
            EthApiError::Unsupported(_) => rpc_err(EthRpcErrorCode::InternalError, msg),
            EthApiError::Internal(_) => rpc_err(EthRpcErrorCode::InternalError, msg),
        }
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
pub fn rpc_err(code: EthRpcErrorCode, msg: impl Into<String>) -> jsonrpsee::types::error::ErrorObject<'static> {
    jsonrpsee::types::error::ErrorObject::owned(code as i32, msg.into(), None::<()>)
}

/// Error related to the Kakarot eth provider
/// which utilizes the starknet provider and
/// a database internally.
#[derive(Debug, Error)]
pub enum KakarotError {
    /// Error related to the starknet provider.
    #[error(transparent)]
    ProviderError(#[from] starknet::providers::ProviderError),
    /// Error related to the database.
    #[error(transparent)]
    DatabaseError(#[from] mongodb::error::Error),
    /// Error related to the evm execution.
    #[error(transparent)]
    ExecutionError(EvmError),
    /// Error related to a starknet call.
    #[error(transparent)]
    CallError(#[from] cainome::cairo_serde::Error),
    /// Error related to starknet to eth conversion or vice versa.
    #[error(transparent)]
    ConversionError(#[from] ConversionError),
}

impl From<KakarotError> for EthApiError {
    fn from(value: KakarotError) -> Self {
        EthApiError::Internal(value)
    }
}

/// Error related to EVM execution.
#[derive(Debug, Error)]
pub enum EvmError {
    #[error("validation failed")]
    ValidationError,
    #[error("state modification error")]
    StateModificationError,
    #[error("unknown opcode")]
    UnknownOpcode,
    #[error("invalid jump dest")]
    InvalidJumpDest,
    #[error("invalid caller")]
    NotKakarotEoaCaller,
    #[error("view function error")]
    ViewFunctionError,
    #[error("stack overflow")]
    StackOverflow,
    #[error("stack underflow")]
    StackUnderflow,
    #[error("out of bounds read")]
    OutOfBoundsRead,
    #[error("unknown precompile {0}")]
    UnknownPrecompile(String),
    #[error("not implemented precompile {0}")]
    NotImplementedPrecompile(String),
    #[error("precompile input error")]
    PrecompileInputError,
    #[error("precompile flag error")]
    PrecompileFlagError,
    #[error("balance error")]
    BalanceError,
    #[error("address collision")]
    AddressCollision,
    #[error("out of gas")]
    OutOfGas,
    #[error("{0}")]
    Other(String),
}

impl From<EvmError> for KakarotError {
    fn from(value: EvmError) -> Self {
        KakarotError::ExecutionError(value)
    }
}

impl From<String> for EvmError {
    fn from(value: String) -> Self {
        let trimmed = value.as_str().trim_start_matches("Kakarot: ").trim_start_matches("Precompile: ");
        match trimmed {
            "eth validation failed" => EvmError::ValidationError,
            "StateModificationError" => EvmError::StateModificationError,
            "UnknownOpcode" => EvmError::UnknownOpcode,
            "invalidJumpDestError" => EvmError::InvalidJumpDest,
            "caller contract is not a Kakarot account" => EvmError::NotKakarotEoaCaller,
            "entrypoint should only be called in view mode" => EvmError::ViewFunctionError,
            "StackOverflow" => EvmError::StackOverflow,
            "StackUnderflow" => EvmError::StackUnderflow,
            "OutOfBoundsRead" => EvmError::OutOfBoundsRead,
            s if s.contains("UnknownPrecompile") => {
                EvmError::UnknownPrecompile(s.trim_start_matches("UnknownPrecompile ").to_string())
            }
            s if s.contains("NotImplementedPrecompile") => {
                EvmError::NotImplementedPrecompile(s.trim_start_matches("NotImplementedPrecompile ").to_string())
            }
            "wrong input_length" => EvmError::PrecompileInputError,
            "flag error" => EvmError::PrecompileFlagError,
            "transfer amount exceeds balance" => EvmError::BalanceError,
            "AddressCollision" => EvmError::AddressCollision,
            s if s.contains("outOfGas") => EvmError::OutOfGas,
            _ => EvmError::Other(value),
        }
    }
}

/// Error related to a transaction.
#[derive(Debug, Error)]
pub enum TransactionError {
    /// Thrown when the chain id is invalid.
    #[error("invalid chain id")]
    InvalidChainId,
    /// Thrown when the gas used overflows u128.
    #[error("gas overflow")]
    GasOverflow,
}

impl TransactionError {
    pub fn error_code(&self) -> EthRpcErrorCode {
        match self {
            TransactionError::InvalidChainId => EthRpcErrorCode::InvalidInput,
            TransactionError::GasOverflow => EthRpcErrorCode::TransactionRejected,
        }
    }
}

/// Error related to signature.
#[derive(Debug, Error)]
pub enum SignatureError {
    /// Thrown when signer recovery fails.
    #[error("could not recover signer")]
    RecoveryError,
    /// Thrown when signing fails.
    #[error("failed to sign")]
    SignError,
    #[error("missing signature")]
    MissingSignature,
}

/// Error related to receipts.
#[derive(Debug, Error)]
pub enum ReceiptError {
    /// Error related to conversion.
    #[error("conversion error")]
    ConversionError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assure_source_error_visible_in_kakarot_error() {
        let err = KakarotError::ProviderError(starknet::providers::ProviderError::StarknetError(
            starknet::core::types::StarknetError::UnexpectedError("test".to_string()),
        ));

        let eth_err: EthApiError = err.into();
        let json_err: ErrorObject<'static> = eth_err.into();

        assert_eq!(json_err.message(), "Internal(ProviderError(StarknetError(UnexpectedError(\"test\"))))");
    }
}
