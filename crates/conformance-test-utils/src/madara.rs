use eyre::Result;
use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
use reth_primitives::{Bytes, U128, U256};
use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;

use crate::types::{ContractAddress, Felt, StorageKey, StorageValue};

pub fn genesis_load_bytecode(
    bytecode: &Bytes,
    address: FieldElement,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    bytecode
        .chunks(16)
        .enumerate()
        .map(|(i, x)| {
            let mut storage_value = [0u8; 16];
            storage_value[..x.len()].copy_from_slice(x);
            let storage_value = u128::from_be_bytes(storage_value);
            let storage_value = FieldElement::from(storage_value).into();

            let storage_key: Felt = get_storage_var_address("bytecode_", &[FieldElement::from(i)]).unwrap().into(); // safe unwrap since bytecode_ is all ascii

            ((address.into(), storage_key), storage_value)
        })
        .collect()
}

/// Generates the genesis storage tuple for setting the storage of a Starknet contract.
///
/// This function calculates the storage key for the storage variable `storage_variable_name` and
/// its keys. The resulting tuple represents the initial storage of the contract, where the storage
/// key at a given `storage_offset` is set to the specified `storage_value`.
pub fn genesis_set_storage_starknet_contract(
    starknet_address: FieldElement,
    storage_variable_name: &str,
    keys: &[FieldElement],
    storage_value: FieldElement,
    storage_offset: u64,
) -> Result<((ContractAddress, StorageKey), StorageValue)> {
    // Compute the storage key for the storage variable name and the keys.
    let mut storage_key = get_storage_var_address(storage_variable_name, keys)?;

    // Add the offset to the storage key.
    storage_key += FieldElement::from(storage_offset);

    let contract_address: ContractAddress = starknet_address.into();

    // Create the tuple for the initial storage data on the Kakarot contract with the given storage key.
    let storage_data = ((contract_address, storage_key.into()), storage_value.into());

    Ok(storage_data)
}

/// Generates the genesis storage tuples for pre-funding a Starknet address on Starknet.
///
/// This function calculates the storage keys for the balance of the ERC20 Fee Token
/// contract using the provided Starknet address. The resulting Vec of tuples represent the initial
/// storage of the Fee Token contract, where the account associated with the Starknet address is
/// pre-funded with the specified `amount`. The `amount` is split into two 128-bit chunks, which
/// are stored in the storage keys at offsets 0 and 1.
pub fn genesis_fund_starknet_address(
    starknet_address: FieldElement,
    amount: U256,
) -> Result<Vec<((ContractAddress, StorageKey), StorageValue)>> {
    // Split the amount into two 128-bit chunks.
    let low = amount & U256::from(U128::MAX);
    let high = amount >> 128;

    // The storage key offsets for the two 128-bit chunks.
    let amount_offset = [(low, 0), (high, 1)]; // (value, offset)

    // Iterate over the storage key offsets and generate the storage tuples.
    amount_offset
        .iter()
        .map(|(value, offset)| {
            genesis_set_storage_starknet_contract(
                FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN)?,
                "ERC20_balances",
                &[starknet_address],
                FieldElement::from_bytes_be(&value.to_be_bytes())?,
                *offset,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
    use kakarot_rpc_core::contracts::contract_account::ContractAccount;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
    use katana_core::backend::state::StorageRecord;
    use reth_primitives::{U128, U256};
    use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, FieldElement};
    use starknet::core::utils::get_storage_var_address;
    use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
    use starknet::providers::JsonRpcClient;
    use starknet_api::core::{ClassHash, ContractAddress as StarknetContractAddress, Nonce};
    use starknet_api::hash::StarkFelt;
    use starknet_api::state::StorageKey as StarknetStorageKey;

    use super::*;

    /// This test verifies that the `genesis_set_storage_starknet_contract` function generates the
    /// correct storage data tuples for a given Starknet address, storage variable name, keys,
    /// storage value, and storage key offset.
    #[tokio::test]
    async fn test_genesis_set_storage_starknet_contract() {
        // Given
        let starknet_address = *ACCOUNT_ADDRESS;
        let storage_variable_name = "test_name";
        let keys = vec![];
        let storage_value = FieldElement::from_hex_be("0x1234").unwrap();
        let storage_offset = 0;

        // This is the expected output tuple of storage data.
        let expected_output = (
            (starknet_address.into(), get_storage_var_address(storage_variable_name, &keys).unwrap().into()),
            storage_value.into(),
        );

        // When
        let result = genesis_set_storage_starknet_contract(
            starknet_address,
            storage_variable_name,
            &keys,
            storage_value,
            storage_offset,
        )
        .unwrap();

        // Then
        assert_eq!(result, expected_output);
    }

    fn get_starknet_storage_key(var_name: &str, args: &[FieldElement]) -> StarknetStorageKey {
        StarknetStorageKey(
            Into::<StarkFelt>::into(get_storage_var_address(var_name, args).unwrap()).try_into().unwrap(),
        )
    }

    #[test]
    fn test_genesis_load_bytecode() {
        // Given
        const TEST_BYTECODE: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        const BIG_ENDIAN_BYTECODE_ONE: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const BIG_ENDIAN_BYTECODE_TWO: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let bytecode = Bytes::from_str(TEST_BYTECODE).unwrap();
        let address = *ACCOUNT_ADDRESS;

        // When
        let storage = genesis_load_bytecode(&bytecode, address);

        // Then
        let expected_storage: Vec<((ContractAddress, StorageKey), StorageValue)> = vec![
            (
                (address.into(), get_storage_var_address("bytecode_", &[FieldElement::from(0u8)]).unwrap().into()),
                FieldElement::from_hex_be(BIG_ENDIAN_BYTECODE_ONE).unwrap().into(),
            ),
            (
                (address.into(), get_storage_var_address("bytecode_", &[FieldElement::from(1u8)]).unwrap().into()),
                FieldElement::from_hex_be(BIG_ENDIAN_BYTECODE_TWO).unwrap().into(),
            ),
        ];
        assert_eq!(expected_storage, storage);
    }

    #[tokio::test]
    async fn test_counter_bytecode() {
        // Given
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        let starknet_client = JsonRpcClient::new(StarknetHttpTransport::new(starknet_test_sequencer.url()));

        // Deploy a counter contract
        let (_, deployed_addresses) =
            deployed_kakarot.deploy_evm_contract(starknet_test_sequencer.url(), "Counter", ()).await.unwrap();
        let deployed_counter = ContractAccount::new(&starknet_client, deployed_addresses.starknet_address);
        let deployed_bytecode = deployed_counter.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();
        let deployed_bytecode_len = deployed_bytecode.len();

        // Use genesis_load_bytecode to get the bytecode to be stored into counter
        let counter_genesis_address = FieldElement::from_str("0x1234").unwrap();
        let counter_genesis_storage = genesis_load_bytecode(&deployed_bytecode, counter_genesis_address);

        // When

        // It is not possible to block the async test task, so we need to spawn a blocking task
        tokio::task::spawn_blocking(move || {
            // Get lock on the Starknet sequencer
            let mut starknet = starknet_test_sequencer.sequencer.starknet.blocking_write();
            let mut counter_storage = HashMap::new();

            // Load the counter bytecode length into the contract
            let key = get_starknet_storage_key("bytecode_len_", &[]);
            let value = Into::<StarkFelt>::into(StarkFelt::from(deployed_bytecode_len as u64));
            counter_storage.insert(key, value);

            // Load the counter bytecode into the contract
            counter_genesis_storage.into_iter().for_each(|((_, k), v)| {
                let key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
                let value = Into::<StarkFelt>::into(v.0);
                counter_storage.insert(key, value);
            });

            // Deploy the contract account at genesis address
            let counter_address =
                StarknetContractAddress(Into::<StarkFelt>::into(counter_genesis_address).try_into().unwrap());
            let counter_storage_record = StorageRecord {
                nonce: Nonce(StarkFelt::from(0u8)),
                class_hash: ClassHash(deployed_kakarot.contract_account_class_hash.into()),
                storage: counter_storage,
            };
            starknet.state.storage.insert(counter_address, counter_storage_record);
        })
        .await
        .unwrap();

        // Create a new counter contract pointing to the genesis initialized storage
        let counter_genesis = ContractAccount::new(&starknet_client, counter_genesis_address);
        let bytecode_actual = counter_genesis.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Then

        // Assert that the expected and actual bytecodes are equal
        assert_eq!(bytecode_actual, deployed_bytecode);
    }

    /// This test verifies that the `genesis_fund_starknet_address` function generates the correct
    /// Vec of storage data tuples for a given Starknet address and amount.
    #[tokio::test]
    async fn test_genesis_fund_starknet_address() {
        // Given
        let starknet_address = FieldElement::from_hex_be("0x1234").unwrap();
        let token_fee_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap();
        let storage_variable_name = "ERC20_balances";
        let amount = U256::MAX;
        let amount_low = amount & U256::from(U128::MAX); // u256.low
        let amount_high = amount >> 128; // u256.high

        // This is equivalent to pre-funding the Starknet address with 2^256 - 1 Fee Tokens.
        // The first storage key is for u256.low
        // The second storage key is for u256.high
        let expected_output = vec![
            (
                (
                    token_fee_address.into(),
                    get_storage_var_address(storage_variable_name, &[starknet_address]).unwrap().into(),
                ),
                FieldElement::from_bytes_be(&amount_low.to_be_bytes()).unwrap().into(),
            ),
            (
                (
                    token_fee_address.into(),
                    (get_storage_var_address(storage_variable_name, &[starknet_address]).unwrap()
                        + FieldElement::from(1u64))
                    .into(),
                ),
                FieldElement::from_bytes_be(&amount_high.to_be_bytes()).unwrap().into(),
            ),
        ];

        // When
        let result = genesis_fund_starknet_address(starknet_address, amount).unwrap();

        // Then
        assert_eq!(result, expected_output);
    }
}
