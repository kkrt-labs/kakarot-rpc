use eyre::Result;
use reth_primitives::{Address, Bloom, Bytes, Signature, TransactionSigned, H160};
use reth_rlp::{Decodable, DecodeError};
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

#[derive(Debug, PartialEq, Eq)]
pub enum FeltOrFeltArray {
    Felt(FieldElement),
    FeltArray(Vec<FieldElement>),
}

struct Calls(Vec<Call>);

/// TryFrom implementation for account contract calls
impl TryFrom<Vec<FieldElement>> for Calls {
    type Error = ValueOutOfRangeError;
    fn try_from(value: Vec<FieldElement>) -> Result<Self, Self::Error> {
        let calls_len = u32::try_from(value[0])? as usize;
        let mut offset = calls_len * 4 + 2;

        let mut calls = vec![];
        for i in 0..calls_len {
            let calldata_len = u32::try_from(value[i * 4 + 4])? as usize;
            let call = Call {
                to: value[i * 4 + 2],
                selector: value[i * 4 + 3],
                calldata: value[offset..offset + calldata_len].to_vec(),
            };
            offset += calldata_len;
            calls.push(call);
        }
        Ok(Calls(calls))
    }
}

/// Returns the decoded return value of the `eth_call` and `eth_send_transaction` entrypoint of
/// Kakarot
pub fn decode_eth_call_return<T: std::error::Error>(
    call_result: &[FieldElement],
) -> Result<Vec<FeltOrFeltArray>, EthApiError<T>> {
    // Parse and decode Kakarot's return data (temporary solution and not scalable - will
    // fail is Kakarot API changes)

    let mut return_data: Vec<FeltOrFeltArray> = vec![FeltOrFeltArray::FeltArray(vec![])];

    let return_data_len = *call_result.first().ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
        entrypoint: "eth_call or eth_send_transaction".into(),
        expected: 1,
        actual: 0,
    })?;

    let mut return_data_len: u64 =
        return_data_len.try_into().map_err(|e: ValueOutOfRangeError| ConversionError::Other(e.to_string()))?;
    let mut counter = 1_usize;

    // Parse call result array
    while return_data_len != 0 {
        let element = call_result.get(counter).ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "eth_call or eth_send_transaction".into(),
            expected: return_data_len as usize,
            actual: counter + 1,
        })?;
        match return_data.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        counter += 1;
        return_data_len -= 1;
    }

    Ok(return_data)
}

pub fn decode_signature_and_to_address_from_tx_calldata(
    calldata: &[FieldElement],
) -> Result<(Signature, Option<Address>), DataDecodingError> {
    let calls =
        Calls::try_from(calldata.to_vec()).map_err(|e| DataDecodingError::SignatureDecodingError(e.to_string()))?;

    let calldata = calls.0[0] // for now we decode signature only from the first call
        .calldata
        .iter()
        .filter_map(|x| u8::try_from(*x).ok())
        .collect::<Vec<u8>>();

    let decoded_tx = TransactionSigned::decode(&mut calldata.as_slice())
        .map_err(|e| DataDecodingError::SignatureDecodingError(e.to_string()))?;

    Ok((decoded_tx.signature, decoded_tx.transaction.to()))
}

#[must_use]
pub fn vec_felt_to_bytes(felt_vec: Vec<FieldElement>) -> Bytes {
    let felt_vec_in_u8: Vec<u8> = felt_vec.into_iter().flat_map(|x| x.to_bytes_be()).collect();
    Bytes::from(felt_vec_in_u8)
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
    use std::str::FromStr;

    use reth_primitives::U256;

    use super::*;

    fn to_vec_field_element(vec: Vec<&str>) -> Vec<FieldElement> {
        vec.into_iter().filter_map(|f| FieldElement::from_hex_be(f).ok()).collect()
    }

    #[test]
    fn test_try_from_calls() {
        let calldata = vec![
            "0x03",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00",
            "0x00a",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00a",
            "0x005",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00f",
            "0x005",
            "0x014",
            "0x000",
            "0x001",
            "0x002",
            "0x003",
            "0x004",
            "0x005",
            "0x006",
            "0x007",
            "0x008",
            "0x009",
            "0x00a",
            "0x00b",
            "0x00c",
            "0x00d",
            "0x00e",
            "0x00f",
            "0x010",
            "0x011",
            "0x012",
            "0x013",
        ];
        let calldata = to_vec_field_element(calldata);
        let calls = Calls::try_from(calldata).unwrap();
        assert_eq!(calls.0.len(), 3);
        let calldata = to_vec_field_element(vec![
            "0x000", "0x001", "0x002", "0x003", "0x004", "0x005", "0x006", "0x007", "0x008", "0x009",
        ]);
        assert_eq!(calls.0[0].calldata, calldata);
        let calldata = to_vec_field_element(vec!["0x00a", "0x00b", "0x00c", "0x00d", "0x00e"]);
        assert_eq!(calls.0[1].calldata, calldata);
        let calldata = to_vec_field_element(vec!["0x00f", "0x010", "0x011", "0x012", "0x013"]);
        assert_eq!(calls.0[2].calldata, calldata);
    }

    #[test]
    fn test_decode_signature_and_to_address_from_tx_calldata() {
        let calldata = vec![
            "0x01",
            "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
            "0x07099f594eb65e00576e1b940a8a735f80bf7604ac401c48627045c4cc286f0",
            "0x00",
            "0x075",
            "0x075",
            "0x02",
            "0x0f8",
            "0x072",
            "0x084",
            "0x04b",
            "0x04b",
            "0x052",
            "0x054",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x082",
            "0x0de",
            "0x0ad",
            "0x084",
            "0x03b",
            "0x09a",
            "0x0ca",
            "0x00",
            "0x094",
            "0x02e",
            "0x011",
            "0x0ed",
            "0x082",
            "0x0f5",
            "0x0ec",
            "0x016",
            "0x05a",
            "0x0b8",
            "0x0ce",
            "0x03c",
            "0x0c0",
            "0x094",
            "0x0f0",
            "0x025",
            "0x0fe",
            "0x075",
            "0x027",
            "0x0f4",
            "0x0d1",
            "0x080",
            "0x084",
            "0x0b3",
            "0x0bc",
            "0x0fa",
            "0x082",
            "0x0c0",
            "0x01",
            "0x0a0",
            "0x088",
            "0x09b",
            "0x0e6",
            "0x07d",
            "0x059",
            "0x0bc",
            "0x01a",
            "0x043",
            "0x0dd",
            "0x080",
            "0x039",
            "0x055",
            "0x0f7",
            "0x091",
            "0x07d",
            "0x0dc",
            "0x0b7",
            "0x0d7",
            "0x048",
            "0x0ed",
            "0x03e",
            "0x09b",
            "0x00",
            "0x0cd",
            "0x0b1",
            "0x059",
            "0x0f2",
            "0x094",
            "0x065",
            "0x019",
            "0x076",
            "0x0b8",
            "0x0a0",
            "0x038",
            "0x01",
            "0x070",
            "0x02a",
            "0x060",
            "0x06f",
            "0x0fb",
            "0x0fd",
            "0x060",
            "0x036",
            "0x04f",
            "0x0f8",
            "0x097",
            "0x0f7",
            "0x0ca",
            "0x051",
            "0x014",
            "0x011",
            "0x0d6",
            "0x066",
            "0x0f",
            "0x093",
            "0x06d",
            "0x0d5",
            "0x01e",
            "0x0b9",
            "0x0a",
            "0x07d",
            "0x030",
            "0x073",
            "0x052",
            "0x061",
        ];
        let calldata = calldata.into_iter().filter_map(|f| FieldElement::from_hex_be(f).ok()).collect::<Vec<_>>();
        let (signature, to) = decode_signature_and_to_address_from_tx_calldata(&calldata).unwrap();
        assert_eq!(
            signature.r,
            U256::from_str("0x889be67d59bc1a43dd803955f7917ddcb7d748ed3e9b00cdb159f294651976b8").unwrap()
        );
        assert_eq!(
            signature.s,
            U256::from_str("0x03801702a606ffbfd60364ff897f7ca511411d6660f936dd51eb90a7d30735261").unwrap()
        );
        assert!(signature.odd_y_parity);
        assert_eq!(to, Some(Address::from_str("0x2e11ed82f5ec165ab8ce3cc094f025fe7527f4d1").unwrap()));
    }

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
}
