#[cfg(not(feature = "hive"))]
use crate::eth_provider::starknet::kakarot_core::MAX_FELTS_IN_CALLDATA;
use crate::{
    eth_provider::{
        error::{EthApiError, SignatureError, TransactionError},
        starknet::kakarot_core::{ETH_SEND_TRANSACTION, KAKAROT_ADDRESS, WHITE_LISTED_EIP_155_TRANSACTION_HASHES},
        utils::split_u256,
    },
    tracing::builder::TRACING_BLOCK_GAS_LIMIT,
};
use alloy_rlp::Encodable;
use reth_primitives::{Transaction, TransactionSigned};
use starknet_crypto::FieldElement;

/// Validates the signed ethereum transaction.
/// The validation checks the following:
/// - The transaction gas limit is lower than the tracing block gas limit.
/// - The transaction chain id (if any) is the same as the one provided.
/// - The transaction hash is whitelisted for pre EIP-155 transactions.
pub(crate) fn validate_transaction(transaction_signed: &TransactionSigned, chain_id: u64) -> Result<(), EthApiError> {
    // If the transaction gas limit is higher than the tracing
    // block gas limit, prevent the transaction from being sent
    // (it will revert anyway on the Starknet side). This assures
    // that all transactions are traceable.
    if transaction_signed.gas_limit() > TRACING_BLOCK_GAS_LIMIT {
        return Err(TransactionError::GasOverflow.into());
    }

    // Recover the signer from the transaction
    let _ = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;

    // Assert the chain is correct
    let maybe_chain_id = transaction_signed.chain_id();
    if !maybe_chain_id.map_or(true, |c| c == chain_id) {
        return Err(TransactionError::InvalidChainId.into());
    }

    // If the transaction is a pre EIP-155 transaction, check if hash is whitelisted
    if maybe_chain_id.is_none() && !WHITE_LISTED_EIP_155_TRANSACTION_HASHES.contains(&transaction_signed.hash) {
        return Err(TransactionError::InvalidTransactionType.into());
    }

    Ok(())
}

/// Returns the transaction's signature as a [`Vec<FieldElement>`].
/// Fields r and s are split into two 16-bytes chunks both converted
/// to [`FieldElement`].
pub(crate) fn transaction_signature_to_field_elements(transaction_signed: &TransactionSigned) -> Vec<FieldElement> {
    let transaction_signature = transaction_signed.signature();

    let mut signature = Vec::with_capacity(5);
    signature.extend_from_slice(&split_u256(transaction_signature.r));
    signature.extend_from_slice(&split_u256(transaction_signature.s));

    // Push the last element of the signature
    // In case of a Legacy Transaction, it is v := {0, 1} + chain_id * 2 + 35
    // or {0, 1} + 27 for pre EIP-155 transactions.
    // Else, it is odd_y_parity
    if let Transaction::Legacy(_) = transaction_signed.transaction {
        let chain_id = transaction_signed.chain_id();
        signature.push(transaction_signature.v(chain_id).into());
    } else {
        signature.push(u64::from(transaction_signature.odd_y_parity).into());
    }

    signature
}

/// Returns the transaction's data RLP encoded without the signature as a [`Vec<FieldElement>`].
/// The data is appended to the Starknet invoke transaction calldata.
///
/// # Example
///
/// For Legacy Transactions: rlp([nonce, `gas_price`, `gas_limit`, to, value, data, `chain_id`, 0, 0])
/// is then converted to a [`Vec<FieldElement>`], packing the data in 31-byte chunks.
#[allow(clippy::unnecessary_wraps)]
pub(crate) fn transaction_data_to_starknet_calldata(
    transaction_signed: &TransactionSigned,
    retries: u8,
) -> Result<Vec<FieldElement>, EthApiError> {
    let mut signed_data = Vec::with_capacity(transaction_signed.transaction.length());
    transaction_signed.transaction.encode_without_signature(&mut signed_data);

    // Pack the calldata in 31-byte chunks
    let mut signed_data: Vec<FieldElement> = std::iter::once(FieldElement::from(signed_data.len()))
        .chain(signed_data.chunks(31).filter_map(|chunk_bytes| FieldElement::from_byte_slice_be(chunk_bytes).ok()))
        .collect();

    // Prepare the calldata for the Starknet invoke transaction
    let capacity = 6 + signed_data.len();

    // Check if call data is too large
    #[cfg(not(feature = "hive"))]
    if capacity > *MAX_FELTS_IN_CALLDATA {
        return Err(EthApiError::CalldataExceededLimit(*MAX_FELTS_IN_CALLDATA, capacity));
    }

    let mut calldata = Vec::with_capacity(capacity);

    // assert that the selector < FieldElement::MAX - retries
    assert!(*ETH_SEND_TRANSACTION < FieldElement::MAX - retries.into());
    let selector = *ETH_SEND_TRANSACTION + retries.into();

    // Retries are used to alter the transaction hash in order to avoid the
    // `DuplicateTx` error from the Starknet gateway, encountered whenever
    // a transaction with the same hash is sent multiple times.
    // We add the retries to the selector in the calldata, since the selector
    // is not used by the EOA contract during the transaction execution.
    calldata.append(&mut vec![
        FieldElement::ONE,        // call array length
        *KAKAROT_ADDRESS,         // contract address
        selector,                 // selector + retries
        FieldElement::ZERO,       // data offset
        signed_data.len().into(), // data length
        signed_data.len().into(), // calldata length
    ]);
    calldata.append(&mut signed_data);

    Ok(calldata)
}
