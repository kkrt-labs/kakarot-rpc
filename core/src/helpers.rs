use eyre::Result;
use reth_primitives::{
    rpc::Log, BlockId as EthBlockId, BlockNumberOrTag, Bloom, Bytes, H160, H256, U128, U256,
};
use reth_rpc_types::TransactionReceipt;

use reth_primitives::Address;

use starknet::{
    accounts::Call,
    core::types::FieldElement,
    providers::jsonrpc::models::{
        BlockId as StarknetBlockId, BlockTag, MaybePendingBlockWithTxHashes,
        MaybePendingBlockWithTxs,
    },
};

use crate::client::{client_api::KakarotClientError, constants::selectors::ETH_SEND_TRANSACTION};

extern crate hex;

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

/// # Errors
///
/// Will return `KakarotClientError` If an error occurs when converting a `Starknet` block hash to a
/// `FieldElement`
pub fn ethers_block_id_to_starknet_block_id(
    block: EthBlockId,
) -> Result<StarknetBlockId, KakarotClientError> {
    match block {
        EthBlockId::Hash(hash) => {
            let hash_felt = FieldElement::from_bytes_be(&hash.block_hash).map_err(|e| {
                KakarotClientError::OtherError(anyhow::anyhow!(
                    "Failed to convert Starknet block hash to FieldElement: {}",
                    e
                ))
            })?;
            Ok(StarknetBlockId::Hash(hash_felt))
        }
        EthBlockId::Number(number) => ethers_block_number_to_starknet_block_id(number),
    }
}

/// # Errors
///
/// TODO: Will return `KakarotClientError`..
pub const fn ethers_block_number_to_starknet_block_id(
    block: BlockNumberOrTag,
) -> Result<StarknetBlockId, KakarotClientError> {
    match block {
        BlockNumberOrTag::Safe | BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized => {
            Ok(StarknetBlockId::Tag(BlockTag::Latest))
        }
        BlockNumberOrTag::Earliest => Ok(StarknetBlockId::Number(0)),
        BlockNumberOrTag::Pending => Ok(StarknetBlockId::Tag(BlockTag::Pending)),
        BlockNumberOrTag::Number(num) => Ok(StarknetBlockId::Number(num)),
    }
}

/// Returns the decoded return value of the `eth_call` entrypoint of Kakarot
pub fn decode_eth_call_return(
    call_result: &[FieldElement],
) -> Result<Vec<FeltOrFeltArray>, KakarotClientError> {
    // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
    // fail is Kakarot API changes)
    // Declare Vec of Result
    let mut segmented_result: Vec<FeltOrFeltArray> = Vec::new();
    let mut tmp_array_len: FieldElement = *call_result.get(0).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call",
        ))
    })?;
    let mut tmp_counter = 1_usize;
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    // Parse first array: stack_accesses
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: return data array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len -= FieldElement::from(1_u64);
    }

    Ok(segmented_result)
}

/// Returns the decoded return value of the `eth_send_transaction` entrypoint
/// of Kakarot
pub fn decode_eth_send_transaction_return(
    call_result: &[FieldElement],
) -> Result<Vec<FeltOrFeltArray>, KakarotClientError> {
    // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
    // fail is Kakarot API changes)
    // Declare Vec of Result
    let mut segmented_result: Vec<FeltOrFeltArray> = Vec::new();
    let mut tmp_array_len: FieldElement = *call_result.get(0).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call",
        ))
    })?;
    let mut tmp_counter = 1_usize;
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    // Parse first array: stack_accesses
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: stack accesses array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len -= FieldElement::from(1_u64);
    }
    // Parse stack_len
    let stack_len = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: stack_len"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*stack_len));
    tmp_counter += 1;
    // Parse second array: memory_accesses
    tmp_array_len = *(call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: memory_accesses_len",
        ))
    })?);
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    tmp_counter += 1;
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: memory accesses array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len -= FieldElement::from(1_u64);
    }
    // Parse memory_len
    let memory_len = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: memory len"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*memory_len));
    tmp_counter += 1;
    // Parse EVM address
    let evm_address = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: evm address"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*evm_address));
    tmp_counter += 1;
    // Parse Starknet Address
    let starknet_address = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: starknet address"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*starknet_address));
    tmp_counter += 1;
    // Parse last array: return_data
    tmp_array_len = *(call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: return_data_len",
        ))
    })?);
    segmented_result.push(FeltOrFeltArray::FeltArray(Vec::new()));
    tmp_counter += 1;
    while tmp_array_len != FieldElement::ZERO {
        let element = call_result.get(tmp_counter).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode return arguments of Kakarot call: return data array",
            ))
        })?;
        match segmented_result.last_mut() {
            Some(FeltOrFeltArray::FeltArray(felt_array)) => felt_array.push(*element),
            Some(FeltOrFeltArray::Felt(_felt)) => (),
            _ => (),
        }
        tmp_counter += 1;
        tmp_array_len -= FieldElement::from(1_u64);
    }
    // Parse gas_used return value
    let gas_used = call_result.get(tmp_counter).ok_or_else(|| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode return arguments of Kakarot call: gas used"
        ))
    })?;
    segmented_result.push(FeltOrFeltArray::Felt(*gas_used));

    Ok(segmented_result)
}

/// # Errors
///
/// TODO: add error case message
pub fn felt_option_to_u256(element: Option<&FieldElement>) -> Result<U256, KakarotClientError> {
    element.map_or_else(
        || Ok(U256::from(0)),
        |x| {
            let inner = x.to_bytes_be();
            Ok(U256::from_be_bytes(inner))
        },
    )
}

#[must_use]
pub fn felt_to_u256(element: FieldElement) -> U256 {
    let inner = element.to_bytes_be();
    U256::from_be_bytes(inner)
}

#[must_use]
pub fn vec_felt_to_bytes(felt_vec: Vec<FieldElement>) -> Bytes {
    let felt_vec_in_u8: Vec<u8> = felt_vec.into_iter().flat_map(|x| x.to_bytes_be()).collect();
    Bytes::from(felt_vec_in_u8)
}

/// Slice the last 20 bytes of the field element and convert it to an Ethereum address
/// ⚠️ BE CAREFUL ⚠️:
/// In order to get the correct/true EVM address of a Kakarot smart contract or account,
/// use the `client.get_evm_address`() method.
/// `starknet_address_to_ethereum_address` is only used for Starknet addresses that do not have an
/// EVM address equivalent.
#[must_use]
pub fn starknet_address_to_ethereum_address(starknet_address: &FieldElement) -> Address {
    H160::from_slice(&starknet_address.to_bytes_be()[12..32])
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
        //TODO: Fetch real data
        cumulative_gas_used: U256::from(1_000_000),
        gas_used: Some(U256::from(500_000)),
        contract_address: None,
        // TODO : default log value
        logs: vec![Log::default()],
        // Bloom is a byte array of length 256
        logs_bloom: Bloom::default(),
        //TODO: Fetch real data
        state_root: None,
        status_code: None,
        //TODO: Fetch real data
        effective_gas_price: U128::from(1_000_000),
        //TODO: Fetch real data
        transaction_type: U256::from(0),
    }
}

/// # Errors
///
/// TODO: add error case message
pub fn hash_to_field_element(hash: H256) -> Result<FieldElement, KakarotClientError> {
    let hash_hex = hex::encode(hash);
    let hash_felt = FieldElement::from_hex_be(&hash_hex).map_err(|e| {
        KakarotClientError::OtherError(anyhow::anyhow!(
            "Failed to convert Starknet block hash to FieldElement: {}",
            e
        ))
    })?;
    Ok(hash_felt)
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
    let calls: Vec<Call> = vec![Call {
        to: kakarot_address,
        selector: ETH_SEND_TRANSACTION,
        calldata: bytes_to_felt_vec(&bytes),
    }];
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
    fn test_decode_eth_send_transaction_return() {
        let call_result = vec![
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000009",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000006661abd",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000007",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0x000000000000000000000000abde1007e67126e0755af0ff0173f919738f8373",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0x062897a9e931ba1ae4721548bd963e3fe67126e0755af0ff0173f919738f8373",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000020",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000000000002",
            )
            .unwrap(),
            FieldElement::from_hex_be(
                "00000000000000000000000000000000000000000000000000000000000fffff",
            )
            .unwrap(),
        ];
        let result = decode_eth_send_transaction_return(&call_result).unwrap();
        assert_eq!(result.len(), 8);
        assert_eq!(
            result[0],
            FeltOrFeltArray::FeltArray(vec![FieldElement::from(1_u64), FieldElement::from(2_u64)])
        );
        assert_eq!(result[1], FeltOrFeltArray::Felt(FieldElement::from(9_u64)));
        assert_eq!(
            result[2],
            FeltOrFeltArray::FeltArray(vec![FieldElement::from_hex_be(
                "0000000000000000000000000000000000000000000000000000000006661abd",
            )
            .unwrap()])
        );
        assert_eq!(result[3], FeltOrFeltArray::Felt(FieldElement::from(7_u64)));
        assert_eq!(
            result[4],
            FeltOrFeltArray::Felt(
                FieldElement::from_hex_be(
                    "0x000000000000000000000000abde1007e67126e0755af0ff0173f919738f8373",
                )
                .unwrap(),
            )
        );
        assert_eq!(
            result[5],
            FeltOrFeltArray::Felt(
                FieldElement::from_hex_be(
                    "0x062897a9e931ba1ae4721548bd963e3fe67126e0755af0ff0173f919738f8373",
                )
                .unwrap(),
            )
        );
        let mut return_data_vec = Vec::new();
        for _ in 0..31 {
            return_data_vec.push(FieldElement::from(0_u64));
        }
        return_data_vec.push(FieldElement::from(2_u64));
        assert_eq!(result[6], FeltOrFeltArray::FeltArray(return_data_vec));
        if let FeltOrFeltArray::FeltArray(felt_array) = &result[6] {
            assert_eq!(felt_array.len(), 32);
        } else {
            panic!("Expected FeltArray of length 32");
        }
        assert_eq!(
            result[7],
            FeltOrFeltArray::Felt(FieldElement::from(0x0000_000f_ffff_u64))
        )
    }
}
