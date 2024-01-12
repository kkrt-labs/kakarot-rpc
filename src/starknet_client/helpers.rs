use reth_primitives::{U128, U256};
use reth_rlp::DecodeError;
use starknet::core::types::FieldElement;
use thiserror::Error;

use crate::starknet_client::constants::selectors::ETH_SEND_TRANSACTION;

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

/// Helper function to split a U256 value into two generic values
/// implementing the From<u128> trait
pub fn split_u256<T: From<u128>>(value: U256) -> [T; 2] {
    let low: u128 = (value & U256::from(U128::MAX)).try_into().unwrap(); // safe to unwrap
    let high: U256 = value >> 128;
    let high: u128 = high.try_into().unwrap(); // safe to unwrap
    [T::from(low), T::from(high)]
}

pub fn try_from_u8_iterator<I: TryInto<u8>, T: FromIterator<u8>>(it: impl Iterator<Item = I>) -> T {
    it.filter_map(|x| TryInto::<u8>::try_into(x).ok()).collect()
}

#[cfg(test)]
mod tests {

    use rstest::*;

    use super::*;

    #[rstest]
    #[test]
    #[case("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")]
    #[case("0x0000000000000000000000000000000000000000000000000000000000000000")]
    #[case("0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")]
    fn test_split_u256(#[case] input: U256) {
        // When
        let result = split_u256::<FieldElement>(input);

        // Then
        // Recalculate the U256 values using the resulting FieldElements
        // The first is the low 128 bits of the U256 value
        // The second is the high 128 bits of the U256 value and is left shifted by 128 bits
        let result: U256 =
            U256::from_be_bytes(result[1].to_bytes_be()) << 128 | U256::from_be_bytes(result[0].to_bytes_be());

        // Assert that the input and recombined U256 values are equal
        assert_eq!(input, result);
    }
}
