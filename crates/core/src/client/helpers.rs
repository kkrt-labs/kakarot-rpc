use eyre::Result;
use reth_primitives::{Bloom, Bytes, H160};
use reth_rlp::DecodeError;
use reth_rpc_types::TransactionReceipt;
use starknet::accounts::Call;
use starknet::core::types::{
    FieldElement, MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs, ValueOutOfRangeError,
};
use thiserror::Error;

use super::constants::{CUMULATIVE_GAS_USED, EFFECTIVE_GAS_PRICE, GAS_USED, TRANSACTION_TYPE};
use crate::client::constants::selectors::ETH_SEND_TRANSACTION;
use crate::client::errors::EthApiError;
use crate::models::ConversionError;

#[derive(Debug, Error)]
pub enum DataDecodingError {
    #[error("failed to decode signature {0}")]
    SignatureDecodingError(String),
    #[error("failed to decode transaction")]
    TransactionDecodingError(#[from] DecodeError),
    #[error("{entrypoint} returned invalid array length, expected {expected}, got {actual}")]
    InvalidReturnArrayLength { entrypoint: String, expected: usize, actual: usize },
}

#[derive(Debug)]
struct InvalidFieldElementError;

impl std::error::Error for InvalidFieldElementError {}

impl std::fmt::Display for InvalidFieldElementError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Invalid FieldElement")
    }
}

pub enum MaybePendingStarknetBlock {
    BlockWithTxHashes(MaybePendingBlockWithTxHashes),
    BlockWithTxs(MaybePendingBlockWithTxs),
}

/// Returns the decoded return value of the `eth_call` entrypoint of Kakarot
pub fn decode_eth_call_return<T: std::error::Error>(
    call_result: &[FieldElement],
) -> Result<Vec<FieldElement>, EthApiError<T>> {
    // Parse and decode Kakarot's return data (temporary solution and not scalable - will
    // fail is Kakarot API changes)

    let return_data_len = *call_result.first().ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
        entrypoint: "eth_call or eth_send_transaction".into(),
        expected: 1,
        actual: 0,
    })?;
    let return_data_len: u64 =
        return_data_len.try_into().map_err(|e: ValueOutOfRangeError| ConversionError::Other(e.to_string()))?;

    let return_data = call_result.get(1..).ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
        entrypoint: "eth_call or eth_send_transaction".into(),
        expected: 2,
        actual: 1,
    })?;

    if return_data.len() != return_data_len as usize {
        return Err(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "eth_call or eth_send_transaction".into(),
            expected: return_data_len as usize,
            actual: return_data.len(),
        }
        .into());
    }

    Ok(return_data.to_vec())
}

#[must_use]
pub fn vec_felt_to_bytes(vec_felt: Vec<FieldElement>) -> Bytes {
    let bytes: Vec<u8> = vec_felt.into_iter().filter_map(|x: FieldElement| u8::try_from(x).ok()).collect();
    Bytes::from(bytes)
}

#[must_use]
pub fn create_default_transaction_receipt() -> TransactionReceipt {
    TransactionReceipt {
        transaction_hash: None,
        transaction_index: None,
        block_hash: None,
        block_number: None,
        from: H160::from(0),
        to: None,
        // TODO: Fetch real data
        cumulative_gas_used: *CUMULATIVE_GAS_USED,
        gas_used: Some(*GAS_USED),
        contract_address: None,
        // TODO : default log value
        logs: vec![],
        // Bloom is a byte array of length 256
        logs_bloom: Bloom::default(),
        // TODO: Fetch real data
        state_root: None,
        status_code: None,
        // TODO: Fetch real data
        effective_gas_price: *EFFECTIVE_GAS_PRICE,
        // TODO: Fetch real data
        transaction_type: *TRANSACTION_TYPE,
    }
}

pub fn bytes_to_felt_vec(bytes: &Bytes) -> Vec<FieldElement> {
    bytes.to_vec().into_iter().map(FieldElement::from).collect()
}

/// Author: <https://github.com/xJonathanLEI/starknet-rs/blob/447182a90839a3e4f096a01afe75ef474186d911/starknet-accounts/src/account/execution.rs#L166>
/// Constructs the calldata for a raw Starknet invoke transaction call
/// ## Arguments
/// * `kakarot_address` - The Kakarot contract address
/// * `bytes` - The calldata to be passed to the contract - RLP encoded raw EVM transaction
///
/// ## Returns
/// * `Vec<FieldElement>` - The calldata for the raw Starknet invoke transaction call
pub fn raw_starknet_calldata(kakarot_address: FieldElement, bytes: Bytes) -> Vec<FieldElement> {
    let calls: Vec<Call> =
        vec![Call { to: kakarot_address, selector: ETH_SEND_TRANSACTION, calldata: bytes_to_felt_vec(&bytes) }];
    let mut concated_calldata: Vec<FieldElement> = vec![];
    let mut execute_calldata: Vec<FieldElement> = vec![calls.len().into()];
    for call in &calls {
        execute_calldata.push(call.to); // to
        execute_calldata.push(call.selector); // selector
        execute_calldata.push(concated_calldata.len().into()); // data_offset
        execute_calldata.push(call.calldata.len().into()); // data_len

        for item in &call.calldata {
            concated_calldata.push(*item);
        }
    }
    execute_calldata.push(concated_calldata.len().into()); // calldata_len
    for item in concated_calldata {
        execute_calldata.push(item); // calldata
    }

    execute_calldata
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_bytes_to_felt_vec() {
        let bytes = Bytes::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let felt_vec = bytes_to_felt_vec(&bytes);
        assert_eq!(felt_vec.len(), 10);
        assert_eq!(
            felt_vec,
            vec![
                FieldElement::from(1_u64),
                FieldElement::from(2_u64),
                FieldElement::from(3_u64),
                FieldElement::from(4_u64),
                FieldElement::from(5_u64),
                FieldElement::from(6_u64),
                FieldElement::from(7_u64),
                FieldElement::from(8_u64),
                FieldElement::from(9_u64),
                FieldElement::from(10_u64)
            ]
        );
    }

    #[test]
    fn test_vec_felt_to_bytes() {
        // Given
        let bytecode: Vec<FieldElement> =
            serde_json::from_str(include_str!("../models/test_data/bytecode/starknet/counter.json")).unwrap();

        // When
        let bytes = vec_felt_to_bytes(bytecode);

        // Then
        let expected: Bytes =
            serde_json::from_str(include_str!("../models/test_data/bytecode/eth/counter.json")).unwrap();
        assert_eq!(expected, bytes);
    }
}
