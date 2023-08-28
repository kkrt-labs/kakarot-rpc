use eyre::Result;
use reth_primitives::{Bloom, Bytes, H160, U128, U256, U64};
use reth_rlp::DecodeError;
use reth_rpc_types::TransactionReceipt;
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
        return_data_len.try_into().map_err(|e: ValueOutOfRangeError| ConversionError::<()>::Other(e.to_string()))?;

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
        // TODO: Compute and return transaction index
        transaction_index: U64::from(0),
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

/// Constructs the calldata for a Kakarot eth_sendRawTransaction call
/// # Arguments
/// * kakarot_address - The address (31-bytes Starknet Address) of the main Kakarot smart contract
/// * eth_calldata - The RLP encoded calldata sent as part of a sendRawTransaction JSON RPC call
/// * transaction_origin - The EVM address of the transaction origin (the caller)
pub fn prepare_kakarot_send_transaction_calldata(
    kakarot_address: FieldElement,
    eth_calldata: Bytes,
) -> Vec<FieldElement> {
    let mut calldata = bytes_to_felt_vec(&eth_calldata);

    let mut execute_calldata: Vec<FieldElement> = vec![
        FieldElement::ONE,                  // call array length
        kakarot_address,                    // contract address
        ETH_SEND_TRANSACTION,               // selector
        FieldElement::ZERO,                 // data offset
        FieldElement::from(calldata.len()), // data length
        FieldElement::from(calldata.len()), // calldata length
    ];
    execute_calldata.append(&mut calldata);

    execute_calldata
}

/// Helper function to split a U256 value into two FieldElements.
pub fn split_u256_into_field_elements(value: U256) -> [FieldElement; 2] {
    let low = value & U256::from(U128::MAX);
    let high = value >> 128;
    [
        FieldElement::from_bytes_be(&low.to_be_bytes()).unwrap(), // Safe unwrap <= U128::MAX.
        FieldElement::from_bytes_be(&high.to_be_bytes()).unwrap(), // Safe unwrap <= U128::MAX.
    ]
}

#[cfg(test)]
mod tests {

    use rstest::*;

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

    #[rstest]
    #[test]
    #[case(
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    )]
    #[case(
        "0x0000000000000000000000000000000000000000000000000000000000000000",
        "0x0000000000000000000000000000000000000000000000000000000000000000"
    )]
    #[case(
        "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
        "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff"
    )]
    fn test_split_u256_into_field_elements(#[case] input: U256, #[case] expected: U256) {
        // When
        let result = split_u256_into_field_elements(input);

        // Then
        // Recalculate the U256 values using the resulting FieldElements
        // The first is the low 128 bits of the U256 value
        // The second is the high 128 bits of the U256 value and is left shifted by 128 bits
        let result: U256 =
            U256::from_be_bytes(result[1].to_bytes_be()) << 128 | U256::from_be_bytes(result[0].to_bytes_be());

        // Assert that the expected and recombined U256 values are equal
        assert_eq!(expected, result);
    }
}
