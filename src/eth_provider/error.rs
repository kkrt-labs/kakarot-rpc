use alloy_sol_types::SolType;
use jsonrpsee::types::ErrorObject;
use reth_primitives::{Bytes, B256};
use starknet_crypto::FieldElement;
use thiserror::Error;

/// List of JSON-RPC error codes from ETH rpc spec.
/// <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-1474.md>
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

impl From<EthApiError> for EthRpcErrorCode {
    fn from(error: EthApiError) -> Self {
        match error {
            EthApiError::UnknownBlock(_) | EthApiError::UnknownBlockNumber(_) | EthApiError::TransactionNotFound(_) => {
                Self::ResourceNotFound
            }
            EthApiError::Signature(_)
            | EthApiError::EthereumDataFormat(_)
            | EthApiError::CalldataExceededLimit(_, _) => Self::InvalidParams,
            EthApiError::Transaction(err) => err.into(),
            EthApiError::Unsupported(_) => Self::InternalError,
            EthApiError::Kakarot(err) => err.into(),
        }
    }
}

/// Error that can occur when interacting with the ETH Api.
#[derive(Error)]
pub enum EthApiError {
    /// When a block is not found
    #[error("unknown block {0}")]
    UnknownBlock(String),
    /// When an unknown block number is encountered
    #[error("unknown block number {0}")]
    UnknownBlockNumber(String),
    /// When a transaction is not found
    #[error("transaction not found {0}")]
    TransactionNotFound(B256),
    /// Error related to transaction
    #[error("transaction error: {0}")]
    Transaction(#[from] TransactionError),
    /// Error related to signing
    #[error("signature error: {0}")]
    Signature(#[from] SignatureError),
    /// Unsupported feature
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// Ethereum data format error
    #[error("ethereum data format error: {0}")]
    EthereumDataFormat(#[from] EthereumDataFormatError),
    /// Other Kakarot error
    #[error("kakarot error: {0}")]
    Kakarot(KakarotError),
    /// Error related to transaction calldata being too large.
    #[error("calldata exceeded limit of {0}: {1}")]
    CalldataExceededLimit(u64, u64),
}

impl std::fmt::Debug for EthApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Kakarot(KakarotError::ProviderError(err)) => {
                write!(f, "starknet provider error: {err:?}")
            }
            _ => write!(f, "{self}"),
        }
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
impl From<EthApiError> for ErrorObject<'static> {
    fn from(value: EthApiError) -> Self {
        let msg = format!("{value:?}");
        ErrorObject::owned(EthRpcErrorCode::from(value) as i32, msg, None::<()>)
    }
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
    /// Error related to the database deserialization.
    #[error(transparent)]
    DatabaseDeserializationError(#[from] mongodb::bson::de::Error),
    /// Error related to the evm execution.
    #[error(transparent)]
    ExecutionError(EvmError),
    /// Error related to a starknet call.
    #[error(transparent)]
    CallError(#[from] cainome::cairo_serde::Error),
}

impl From<KakarotError> for EthApiError {
    fn from(value: KakarotError) -> Self {
        Self::Kakarot(value)
    }
}

impl From<KakarotError> for EthRpcErrorCode {
    fn from(value: KakarotError) -> Self {
        match value {
            KakarotError::ExecutionError(_) => Self::ExecutionError,
            _ => Self::InternalError,
        }
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
        Self::ExecutionError(value)
    }
}

impl From<Vec<FieldElement>> for EvmError {
    fn from(value: Vec<FieldElement>) -> Self {
        let bytes = value.into_iter().filter_map(|x| u8::try_from(x).ok()).collect::<Vec<_>>();
        let maybe_revert_reason = String::from_utf8(bytes.clone());
        if maybe_revert_reason.is_err() {
            return Self::Other(decode_err(&bytes));
        }

        let revert_reason = maybe_revert_reason.unwrap(); // safe unwrap
        let trimmed = revert_reason.trim_start_matches("Kakarot: ").trim_start_matches("Precompile: ");
        match trimmed {
            "eth validation failed" => Self::ValidationError,
            "StateModificationError" => Self::StateModificationError,
            "UnknownOpcode" => Self::UnknownOpcode,
            "invalidJumpDestError" => Self::InvalidJumpDest,
            "caller contract is not a Kakarot account" => Self::NotKakarotEoaCaller,
            "entrypoint should only be called in view mode" => Self::ViewFunctionError,
            "StackOverflow" => Self::StackOverflow,
            "StackUnderflow" => Self::StackUnderflow,
            "OutOfBoundsRead" => Self::OutOfBoundsRead,
            s if s.contains("UnknownPrecompile") => {
                Self::UnknownPrecompile(s.trim_start_matches("UnknownPrecompile ").to_string())
            }
            s if s.contains("NotImplementedPrecompile") => {
                Self::NotImplementedPrecompile(s.trim_start_matches("NotImplementedPrecompile ").to_string())
            }
            "wrong input_length" => Self::PrecompileInputError,
            "flag error" => Self::PrecompileFlagError,
            "transfer amount exceeds balance" => Self::BalanceError,
            "AddressCollision" => Self::AddressCollision,
            s if s.contains("outOfGas") => Self::OutOfGas,
            _ => Self::Other(decode_err(&bytes)),
        }
    }
}

fn decode_err(bytes: &[u8]) -> String {
    // Skip the first 4 bytes which is the function selector
    let msg = &bytes[4..];
    let maybe_decoded_msg = alloy_sol_types::sol_data::String::abi_decode(msg, true);
    maybe_decoded_msg.map_or_else(|_| format!("{}", bytes.iter().collect::<Bytes>()), |s| s)
}

/// Error related to a transaction.
#[derive(Debug, Error)]
pub enum TransactionError {
    /// Thrown when the chain id is invalid.
    #[error("invalid chain id")]
    InvalidChainId,
    /// Thrown when the transaction type is invalid.
    #[error("invalid transaction type")]
    InvalidTransactionType,
    /// Thrown when the gas used overflows u128.
    #[error("gas overflow")]
    GasOverflow,
    /// Thrown when the transaction isn't the
    /// BlockTransactions::FullTransactions variant.
    #[error("expected full transactions")]
    ExpectedFullTransactions,
    /// Thrown if the tracing fails
    #[error("tracing error: {0}")]
    Tracing(Box<dyn std::error::Error + Send + Sync>),
}

impl From<TransactionError> for EthRpcErrorCode {
    fn from(error: TransactionError) -> Self {
        match error {
            TransactionError::InvalidChainId | TransactionError::InvalidTransactionType => Self::InvalidInput,
            TransactionError::GasOverflow => Self::TransactionRejected,
            TransactionError::ExpectedFullTransactions | TransactionError::Tracing(_) => Self::InternalError,
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
    /// Thrown when signature is missing.
    #[error("missing signature")]
    MissingSignature,
    /// Thrown when parity is invalid.
    #[error("invalid parity")]
    InvalidParity,
}

/// Error related to Ethereum data format.
#[derive(Debug, Error)]
pub enum EthereumDataFormatError {
    /// Error related to conversion in header.
    #[error("header conversion error")]
    HeaderConversionError,
    /// Error related to conversion in receipt.
    #[error("header conversion error")]
    ReceiptConversionError,
    /// Error related to conversion in transaction.
    #[error("transaction conversion error")]
    TransactionConversionError,
    /// Error related to starknet to eth conversion or vice versa.
    #[error("primitive conversion error")]
    PrimitiveError,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_assure_source_error_visible_in_kakarot_error() {
        // Given
        let err = KakarotError::ProviderError(starknet::providers::ProviderError::StarknetError(
            starknet::core::types::StarknetError::UnexpectedError("test".to_string()),
        ));

        // When
        let eth_err: EthApiError = err.into();
        let json_err: ErrorObject<'static> = eth_err.into();

        // Then
        assert_eq!(json_err.message(), "starknet provider error: StarknetError(UnexpectedError(\"test\"))");
    }

    #[test]
    fn test_decode_evm_error() {
        // Given
        let bytes: Vec<_> = vec![
            0x08u8, 0xc3, 0x79, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17, 0x46, 0x61, 0x75,
            0x63, 0x65, 0x74, 0x3a, 0x20, 0x43, 0x6c, 0x61, 0x69, 0x6d, 0x20, 0x74, 0x6f, 0x6f, 0x20, 0x73, 0x6f, 0x6f,
            0x6e, 0x2e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ]
        .into_iter()
        .map(FieldElement::from)
        .collect();

        // When
        let evm_err: EvmError = bytes.into();

        // Then
        if let EvmError::Other(err) = evm_err {
            assert_eq!(err, "Faucet: Claim too soon.");
        } else {
            panic!("Expected EvmError::Other, got {evm_err:?}");
        }
    }
}
