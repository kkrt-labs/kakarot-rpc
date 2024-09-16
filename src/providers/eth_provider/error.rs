use alloy_sol_types::decode_revert_reason;
use jsonrpsee::types::ErrorObject;
use num_traits::cast::ToPrimitive;
use reth_primitives::{Bytes, B256};
use reth_rpc_eth_types::EthApiError as RethEthApiError;
use reth_rpc_types::{BlockHashOrNumber, ToRpcError};
use reth_transaction_pool::error::PoolError;
use starknet::core::types::Felt;
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

impl From<&EthApiError> for EthRpcErrorCode {
    fn from(error: &EthApiError) -> Self {
        match error {
            EthApiError::UnknownBlock(_) | EthApiError::UnknownBlockNumber(_) | EthApiError::TransactionNotFound(_) => {
                Self::ResourceNotFound
            }
            EthApiError::Signature(_)
            | EthApiError::EthereumDataFormat(_)
            | EthApiError::CalldataExceededLimit(_, _)
            | EthApiError::RethEthApi(_) => Self::InvalidParams,
            EthApiError::Transaction(err) => err.into(),
            // TODO improve the error
            EthApiError::Unsupported(_) | EthApiError::Kakarot(_) | EthApiError::Pool(_) => Self::InternalError,
            EthApiError::Execution(_) => Self::ExecutionError,
        }
    }
}

impl From<EthApiError> for RethEthApiError {
    fn from(value: EthApiError) -> Self {
        Self::other(value)
    }
}

/// Error that can occur when interacting with the ETH Api.
#[derive(Debug, Error)]
pub enum EthApiError {
    /// When a block is not found
    UnknownBlock(BlockHashOrNumber),
    /// When an unknown block number is encountered
    UnknownBlockNumber(Option<u64>),
    /// When a transaction is not found
    TransactionNotFound(B256),
    /// Error related to transaction
    Transaction(#[from] TransactionError),
    /// Error related to transaction pool
    Pool(#[from] PoolError),
    /// Error related to signing
    Signature(#[from] SignatureError),
    /// Unsupported feature
    Unsupported(&'static str),
    /// Ethereum data format error
    EthereumDataFormat(#[from] EthereumDataFormatError),
    /// Execution error
    Execution(#[from] ExecutionError),
    /// Kakarot related error (database, ...)
    Kakarot(KakarotError),
    /// Error related to transaction calldata being too large.
    CalldataExceededLimit(usize, usize),
    /// Reth Eth API error
    RethEthApi(#[from] RethEthApiError),
}

impl std::fmt::Display for EthApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownBlock(block) => write!(f, "unknown block {block}"),
            Self::UnknownBlockNumber(block) => write!(f, "unknown block number {block:?}"),
            Self::TransactionNotFound(tx) => write!(f, "transaction not found {tx}"),
            Self::Transaction(err) => write!(f, "{err}"),
            Self::Pool(err) => write!(f, "{err}"),
            Self::Signature(err) => write!(f, "{err}"),
            Self::RethEthApi(err) => write!(f, "{err}"),
            Self::Unsupported(feature) => write!(f, "unsupported: {feature}"),
            Self::EthereumDataFormat(err) => write!(f, "ethereum data format error: {err}"),
            Self::Execution(err) => write!(f, "{err}"),
            Self::Kakarot(KakarotError::Provider(err)) => {
                // We use Debug here otherwise we risk losing some information on contract error
                write!(f, "starknet provider error: {err:?}")
            }
            Self::Kakarot(err) => write!(f, "kakarot error: {err}"),
            Self::CalldataExceededLimit(limit, actual) => {
                write!(f, "calldata exceeded limit of {limit}: {actual}")
            }
        }
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
impl From<EthApiError> for ErrorObject<'static> {
    fn from(value: EthApiError) -> Self {
        (&value).into()
    }
}

/// Constructs a JSON-RPC error object, consisting of `code` and `message`.
impl From<&EthApiError> for ErrorObject<'static> {
    fn from(value: &EthApiError) -> Self {
        let msg = format!("{value}");
        let code = EthRpcErrorCode::from(value);
        let data = match value {
            EthApiError::Execution(ExecutionError::Evm(EvmError::Other(ref b))) => Some(b.clone()),
            _ => None,
        };
        ErrorObject::owned(code as i32, msg, data)
    }
}

impl ToRpcError for EthApiError {
    fn to_rpc_error(&self) -> ErrorObject<'static> {
        self.into()
    }
}

/// Error related to the Kakarot eth provider
/// which utilizes the starknet provider and
/// a database internally.
#[derive(Debug, Error)]
pub enum KakarotError {
    /// Error related to the starknet provider.
    #[error(transparent)]
    Provider(#[from] starknet::providers::ProviderError),
    /// Error related to the database.
    #[error(transparent)]
    Database(#[from] mongodb::error::Error),
    /// Error related to the database deserialization.
    #[error(transparent)]
    DatabaseDeserialization(#[from] mongodb::bson::de::Error),
}

impl From<KakarotError> for EthApiError {
    fn from(value: KakarotError) -> Self {
        Self::Kakarot(value)
    }
}

/// Error related to execution errors, by the EVM or Cairo vm.
#[derive(Debug, Error)]
pub enum ExecutionError {
    /// Error related to the EVM execution failures.
    Evm(#[from] EvmError),
    /// Error related to the Cairo vm execution failures.
    CairoVm(#[from] CairoError),
    /// Other execution error.
    Other(String),
}

impl From<cainome::cairo_serde::Error> for ExecutionError {
    fn from(error: cainome::cairo_serde::Error) -> Self {
        let error = error.to_string();
        if error.contains("RunResources has no remaining steps.") {
            return Self::CairoVm(CairoError::VmOutOfResources);
        }
        Self::Other(error)
    }
}

impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("execution reverted")?;
        match self {
            Self::Evm(err) => match err {
                EvmError::Other(b) => {
                    if let Some(reason) = decode_revert_reason(b.as_ref()) {
                        write!(f, ": {reason}")?;
                    }
                    Ok(())
                }
                _ => write!(f, ": {err}"),
            },
            Self::CairoVm(err) => write!(f, ": {err}"),
            Self::Other(err) => write!(f, ": {err}"),
        }
    }
}

/// Error related to the Cairo vm execution failures.
#[derive(Debug, Error)]
pub enum CairoError {
    #[error("cairo vm out of resources")]
    VmOutOfResources,
}

/// Error related to EVM execution.
#[derive(Debug, Error)]
pub enum EvmError {
    #[error("validation failed")]
    Validation,
    #[error("state modification error")]
    StateModification,
    #[error("unknown opcode")]
    UnknownOpcode,
    #[error("invalid jump dest")]
    InvalidJumpDest,
    #[error("caller is not a Kakarot EOA")]
    NotKakarotEoaCaller,
    #[error("function limited to view call")]
    ViewFunction,
    #[error("stack overflow")]
    StackOverflow,
    #[error("stack underflow")]
    StackUnderflow,
    #[error("out of bounds read")]
    OutOfBoundsRead,
    #[error("unknown precompile {0}")]
    UnknownPrecompile(String),
    #[error("unauthorized precompile")]
    UnauthorizedPrecompile,
    #[error("not implemented precompile {0}")]
    NotImplementedPrecompile(String),
    #[error("invalid cairo selector")]
    InvalidCairoSelector,
    #[error("precompile wrong input length")]
    PrecompileInputLength,
    #[error("precompile flag error")]
    PrecompileFlag,
    #[error("transfer amount exceeds balance")]
    Balance,
    #[error("address collision")]
    AddressCollision,
    #[error("out of gas")]
    OutOfGas,
    #[error("{0}")]
    Other(Bytes),
}

impl From<Vec<Felt>> for EvmError {
    fn from(value: Vec<Felt>) -> Self {
        let bytes = value.into_iter().filter_map(|x| x.to_u8()).collect::<Vec<_>>();
        let maybe_revert_reason = String::from_utf8(bytes.clone());
        if maybe_revert_reason.is_err() {
            return Self::Other(bytes.into());
        }

        let revert_reason = maybe_revert_reason.unwrap(); // safe unwrap
        let trimmed = revert_reason.trim_start_matches("Kakarot: ").trim_start_matches("Precompile: ");
        match trimmed {
            "eth validation failed" => Self::Validation,
            "StateModificationError" => Self::StateModification,
            "UnknownOpcode" => Self::UnknownOpcode,
            "invalidJumpDestError" => Self::InvalidJumpDest,
            "caller contract is not a Kakarot account" => Self::NotKakarotEoaCaller,
            "entrypoint should only be called in view mode" => Self::ViewFunction,
            "StackOverflow" => Self::StackOverflow,
            "StackUnderflow" => Self::StackUnderflow,
            "OutOfBoundsRead" => Self::OutOfBoundsRead,
            s if s.contains("UnknownPrecompile") => {
                Self::UnknownPrecompile(s.trim_start_matches("UnknownPrecompile ").to_string())
            }
            "unauthorizedPrecompile" => Self::UnauthorizedPrecompile,
            s if s.contains("NotImplementedPrecompile") => {
                Self::NotImplementedPrecompile(s.trim_start_matches("NotImplementedPrecompile ").to_string())
            }
            "invalidCairoSelector" => Self::InvalidCairoSelector,
            "wrong input_length" => Self::PrecompileInputLength,
            "flag error" => Self::PrecompileFlag,
            "transfer amount exceeds balance" => Self::Balance,
            "addressCollision" => Self::AddressCollision,
            s if s.contains("outOfGas") => Self::OutOfGas,
            _ => Self::Other(bytes.into()),
        }
    }
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
    /// Thrown when the max fee per gas is lower than the base fee.
    #[error("max fee per gas {0} lower than base fee {1}")]
    FeeCapTooLow(u128, u128),
    /// Thrown when the max fee per gas is lower than the max priority fee per gas.
    #[error("max fee per gas {0} lower than max priority fee per gas {1}")]
    TipAboveFeeCap(u128, u128),
    /// Thrown when the gas limit exceeds the block's gas limit.
    #[error("transaction gas limit {0} exceeds block gas limit {1}")]
    ExceedsBlockGasLimit(u128, u128),
    /// Thrown when the transaction isn't the
    /// [`BlockTransactions::FullTransactions`] variant.
    #[error("expected full transactions")]
    ExpectedFullTransactions,
    /// Thrown if the broadcasting of the Starknet transaction fails
    #[error("broadcasting error: {0}")]
    Broadcast(Box<dyn std::error::Error + Send + Sync>),
    /// Thrown if the tracing fails
    #[error("tracing error: {0}")]
    Tracing(Box<dyn std::error::Error + Send + Sync>),
    /// Thrown if the call with state or block overrides fails
    #[error("tracing error: {0}")]
    Call(Box<dyn std::error::Error + Send + Sync>),
}

impl From<&TransactionError> for EthRpcErrorCode {
    fn from(error: &TransactionError) -> Self {
        match error {
            TransactionError::InvalidChainId | TransactionError::InvalidTransactionType => Self::InvalidInput,
            TransactionError::GasOverflow
            | TransactionError::FeeCapTooLow(_, _)
            | TransactionError::TipAboveFeeCap(_, _) => Self::TransactionRejected,
            TransactionError::ExpectedFullTransactions
            | TransactionError::Tracing(_)
            | TransactionError::Call(_)
            | TransactionError::Broadcast(_)
            | TransactionError::ExceedsBlockGasLimit(_, _) => Self::InternalError,
        }
    }
}

/// Error related to signature.
#[derive(Debug, Error)]
pub enum SignatureError {
    /// Thrown when signer recovery fails.
    #[error("could not recover signer")]
    Recovery,
    /// Thrown when signing fails.
    #[error("failed to sign transaction")]
    SigningFailure,
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
    HeaderConversion,
    /// Error related to conversion in receipt.
    #[error("header conversion error")]
    ReceiptConversion,
    /// Error related to conversion in transaction.
    #[error("transaction conversion error")]
    TransactionConversion,
    /// Error related to starknet to eth conversion or vice versa.
    #[error("primitive conversion error")]
    Primitive,
}

#[cfg(test)]
mod tests {
    use starknet::core::types::ContractErrorData;

    use super::*;

    #[test]
    fn test_assure_source_error_visible_in_kakarot_error() {
        // Given
        let err = KakarotError::Provider(starknet::providers::ProviderError::StarknetError(
            starknet::core::types::StarknetError::UnexpectedError("test".to_string()),
        ));

        // When
        let eth_err: EthApiError = err.into();
        let json_err: ErrorObject<'static> = eth_err.into();

        // Then
        assert_eq!(json_err.message(), "starknet provider error: StarknetError(UnexpectedError(\"test\"))");
    }

    #[test]
    fn test_decode_revert_message() {
        // Given
        let b: Vec<_> = vec![
            0x08u8, 0xc3, 0x79, 0xa0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17, 0x46, 0x61, 0x75,
            0x63, 0x65, 0x74, 0x3a, 0x20, 0x43, 0x6c, 0x61, 0x69, 0x6d, 0x20, 0x74, 0x6f, 0x6f, 0x20, 0x73, 0x6f, 0x6f,
            0x6e, 0x2e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let bytes = b.clone().into_iter().map(Felt::from).collect::<Vec<_>>();

        // When
        let evm_err: EvmError = bytes.into();
        let json_rpsee_error: ErrorObject<'static> = EthApiError::Execution(ExecutionError::Evm(evm_err)).into();

        // Then
        assert_eq!(json_rpsee_error.message(), "execution reverted: revert: Faucet: Claim too soon.");
        assert_eq!(json_rpsee_error.code(), 3);
        assert_eq!(format!("{}", json_rpsee_error.data().unwrap()), format!("\"{}\"", Bytes::from(b)));
    }

    #[test]
    fn test_decode_undecodable_message() {
        // Given
        let b = vec![
            0x6cu8, 0xa7, 0xb8, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x71,
            0x52, 0xe0, 0x85, 0x5b, 0xab, 0x82, 0xb8, 0xe1, 0x0b, 0x86, 0x92, 0xe5, 0x84, 0xad, 0x03, 0x4b, 0xd2, 0x29,
            0x12,
        ];
        let bytes = b.clone().into_iter().map(Felt::from).collect::<Vec<_>>();

        // When
        let evm_err: EvmError = bytes.into();
        let json_rpsee_error: ErrorObject<'static> = EthApiError::Execution(ExecutionError::Evm(evm_err)).into();

        // Then
        assert_eq!(json_rpsee_error.message(), "execution reverted");
        assert_eq!(json_rpsee_error.code(), 3);
        assert_eq!(format!("{}", json_rpsee_error.data().unwrap()), format!("\"{}\"", Bytes::from(b)));
    }

    #[test]
    fn test_decode_kakarot_evm_error() {
        // Given
        let bytes = vec![
            0x4bu8, 0x61, 0x6b, 0x61, 0x72, 0x6f, 0x74, 0x3a, 0x20, 0x65, 0x6e, 0x74, 0x72, 0x79, 0x70, 0x6f, 0x69,
            0x6e, 0x74, 0x20, 0x73, 0x68, 0x6f, 0x75, 0x6c, 0x64, 0x20, 0x6f, 0x6e, 0x6c, 0x79, 0x20, 0x62, 0x65, 0x20,
            0x63, 0x61, 0x6c, 0x6c, 0x65, 0x64, 0x20, 0x69, 0x6e, 0x20, 0x76, 0x69, 0x65, 0x77, 0x20, 0x6d, 0x6f, 0x64,
            0x65,
        ]
        .into_iter()
        .map(Felt::from)
        .collect::<Vec<_>>();

        // When
        let evm_err: EvmError = bytes.into();
        let json_rpsee_error: ErrorObject<'static> = EthApiError::Execution(ExecutionError::Evm(evm_err)).into();

        // Then
        assert_eq!(json_rpsee_error.message(), "execution reverted: function limited to view call");
        assert_eq!(json_rpsee_error.code(), 3);
        assert!(json_rpsee_error.data().is_none());
    }

    #[test]
    fn test_display_execution_error() {
        // Given
        let err = EthApiError::Execution(ExecutionError::Evm(EvmError::Balance));

        // When
        let display = format!("{err}");

        // Then
        assert_eq!(display, "execution reverted: transfer amount exceeds balance");
    }

    #[test]
    fn test_from_run_resources_error() {
        let err = cainome::cairo_serde::Error::Provider(starknet::providers::ProviderError::StarknetError(
            starknet::core::types::StarknetError::ContractError(ContractErrorData {
                revert_error:
                    "Error in the called contract (0x007fbaddebb5e88696fac9fc5aaf8bdff4bbca1eaf06a0cb5ae94df8ea93f882):
                     Error at pc=0:31:
                     Got an exception while executing a hint.
                     Cairo traceback (most recent call last):
                     Unknown location (pc=0:4836)
                     Unknown location (pc=0:4775)
                     Unknown location (pc=0:3860)
                     Unknown location (pc=0:663)
                     Error in the called contract (0x040e005e7acea50434c537ba62e72e8a8e960d679c87609029d4639e2bdb9cb2):
                     Error at pc=0:24:
                     Could not reach the end of the program. RunResources has no remaining steps.
                     Cairo traceback (most recent call last):
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23327)
                     Unknown location (pc=0:23325)
                     Unknown location (pc=0:22158)"
                        .to_string(),
            }),
        ));

        // When
        let eth_err: ExecutionError = err.into();
        let display = format!("{eth_err}");

        // Then
        assert_eq!(display, "execution reverted: cairo vm out of resources");
    }
}
