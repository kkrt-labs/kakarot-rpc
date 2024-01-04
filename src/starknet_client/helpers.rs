use eyre::Result;
use reth_primitives::{U128, U256};
use reth_rlp::DecodeError;
use starknet::core::types::{
    FieldElement, MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs, ValueOutOfRangeError,
};
use thiserror::Error;

use crate::models::errors::ConversionError;
use crate::starknet_client::constants::selectors::ETH_SEND_TRANSACTION;
use crate::starknet_client::errors::EthApiError;

#[derive(Debug, Error)]
pub enum DataDecodingError {
    #[error("failed to decode signature {0}")]
    SignatureDecodingError(String),
    #[error("failed to decode calldata {0}")]
    CalldataDecodingError(String),
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
pub fn decode_eth_call_return(call_result: &[FieldElement]) -> Result<Vec<FieldElement>, EthApiError> {
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

/// Constructs the calldata for a raw Starknet invoke transaction call
pub fn prepare_kakarot_eth_send_transaction(
    kakarot_address: FieldElement,
    mut calldata: Vec<FieldElement>,
) -> Vec<FieldElement> {
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
    let high: U256 = value >> 128;
    [
        FieldElement::from_bytes_be(&low.to_be_bytes()).unwrap(), // Safe unwrap <= U128::MAX.
        FieldElement::from_bytes_be(&high.to_be_bytes()).unwrap(), // Safe unwrap <= U128::MAX.
    ]
}

#[cfg(test)]
mod tests {

    use rstest::*;

    use super::*;

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
