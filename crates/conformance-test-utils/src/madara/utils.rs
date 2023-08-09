use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
use reth_primitives::{Bytes, U128, U256};
use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;

use crate::types::{ContractAddress, StorageKey, StorageValue};

/// Generates the genesis storage tuples for setting the bytecode of a Kakarot countract account
///
/// This function calculates the storage keys for the Kakarot contract using the provided bytecode
/// and Starknet address. The resulting Vec of tuples represent the initial storage of the Kakarot
/// contract, where the storage key is computed using the storage variable "bytecode_" and the index
/// of the 16-byte chunk of the bytecode. The value stored is the 16-byte chunk of the bytecode.
pub fn genesis_set_bytecode(
    bytecode: &Bytes,
    starknet_address: FieldElement,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    bytecode
        .chunks(16)
        .enumerate()
        .map(|(i, x)| {
            let mut storage_value = [0u8; 16];
            storage_value[..x.len()].copy_from_slice(x);
            let storage_value = FieldElement::from(u128::from_be_bytes(storage_value));

            genesis_set_storage_starknet_contract(
                starknet_address,
                "bytecode_",
                &[FieldElement::from(i)],
                storage_value,
                0, // only felt is stored so offset is always 0
            )
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
) -> ((ContractAddress, StorageKey), StorageValue) {
    // Compute the storage key for the storage variable name and the keys.
    let mut storage_key =
        get_storage_var_address(storage_variable_name, keys).expect("Non-ASCII storage variable name");

    // Add the offset to the storage key.
    storage_key += FieldElement::from(storage_offset);

    let contract_address: ContractAddress = starknet_address.into();

    // Create the tuple for the initial storage data on the Starknet contract with the given storage
    // key.
    ((contract_address, storage_key.into()), storage_value.into())
}

/// Generates the genesis storage tuples for pre-funding a Starknet address on Madara.
///
/// This function calculates the storage keys for the balance of the ERC20 Fee Token
/// contract using the provided Starknet address. The resulting Vec of tuples represent the initial
/// storage of the Fee Token contract, where the account associated with the Starknet address is
/// pre-funded with the specified `amount`. The `amount` is split into two 128-bit chunks, which
/// are stored in the storage keys at offsets 0 and 1.
pub fn genesis_fund_starknet_address(
    starknet_address: FieldElement,
    amount: U256,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    // Split the amount into two 128-bit chunks.
    let amount = split_u256_into_field_elements(amount);

    // Iterate over the storage key offsets and generate the storage tuples.
    amount
        .iter()
        .enumerate() // Enumerate the key offsets.
        .map(|(offset, value)| {
            genesis_set_storage_starknet_contract(
                FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(), // Safe unwrap
                "ERC20_balances",
                &[starknet_address],
                *value,
                offset as u64,
            )
        })
        .collect()
}

/// Generates the genesis storage tuples for setting the storage of the Kakarot contract.
///
/// This function calculates the storage keys for the Kakarot contract using the provided Starknet
/// address. The resulting Vec of tuples represent the initial storage of the Kakarot contract,
/// where the storage key is computed using the provided `key` of the storage variable "storage_"
/// and the `value` is split into two 128-bit chunks, which are stored in the storage keys at
/// offsets 0 and 1.
pub fn genesis_set_storage_kakarot_contract_account(
    starknet_address: FieldElement,
    key: U256,
    value: U256,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    // Split the key into Vec of two 128-bit chunks.
    let keys = split_u256_into_field_elements(key);

    // Split the value into two 128-bit chunks.
    let values = split_u256_into_field_elements(value);

    // Iterate over the storage key offsets and generate the storage tuples.
    values
        .iter()
        .enumerate() // Enumerate the key offsets.
        .map(|(offset, value)| {
            genesis_set_storage_starknet_contract(
                starknet_address,
                "storage_",
                &keys,
                *value,
                offset as u64,
            )
        })
        .collect()
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
    use std::collections::HashMap;
    use std::str::FromStr;
    use std::sync::Arc;

    use kakarot_rpc_core::client::api::KakarotStarknetApi;
    use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
    use kakarot_rpc_core::contracts::account::Account;
    use kakarot_rpc_core::contracts::contract_account::ContractAccount;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::deploy_helpers::{ContractDeploymentArgs, KakarotTestEnvironment};
    use katana_core::backend::state::StorageRecord;
    use reth_primitives::U256;
    use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, FieldElement};
    use starknet::core::utils::get_storage_var_address;
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
        let starknet_address = FieldElement::from_hex_be("0x1234").unwrap();
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
        );

        // Then
        assert_eq!(result, expected_output);
    }

    fn get_starknet_storage_key(var_name: &str, args: &[FieldElement]) -> StarknetStorageKey {
        StarknetStorageKey(
            Into::<StarkFelt>::into(get_storage_var_address(var_name, args).unwrap()).try_into().unwrap(),
        )
    }

    #[test]
    fn test_genesis_set_bytecode() {
        // Given
        const TEST_BYTECODE: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        const BIG_ENDIAN_BYTECODE_ONE: &str = "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        const BIG_ENDIAN_BYTECODE_TWO: &str = "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let bytecode = Bytes::from_str(TEST_BYTECODE).unwrap();
        let address = *ACCOUNT_ADDRESS;

        // When
        let storage = genesis_set_bytecode(&bytecode, address);

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
        let test_environment = Arc::new(
            KakarotTestEnvironment::new()
                .await
                .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
                .await,
        );
        let starknet_client = test_environment.client().starknet_provider();
        let counter = test_environment.evm_contract("Counter");
        let counter_contract = ContractAccount::new(counter.addresses.starknet_address, &starknet_client);

        // When
        let deployed_bytecode = counter_contract.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();
        let deployed_bytecode_len = deployed_bytecode.len();

        // Use genesis_set_bytecode to get the bytecode to be stored into counter
        let counter_genesis_address = FieldElement::from_str("0x1234").unwrap();
        let counter_genesis_storage = genesis_set_bytecode(&deployed_bytecode, counter_genesis_address);

        // Create an atomic reference to the test environment to avoid dropping it
        let env = Arc::clone(&test_environment);
        // It is not possible to block the async test task, so we need to spawn a blocking task
        tokio::task::spawn_blocking(move || {
            // Get lock on the Starknet sequencer
            let mut starknet = env.sequencer().sequencer.starknet.blocking_write();
            let mut counter_storage = HashMap::new();

            // Set the counter bytecode length into the contract
            let key = get_starknet_storage_key("bytecode_len_", &[]);
            let value = Into::<StarkFelt>::into(StarkFelt::from(deployed_bytecode_len as u64));
            counter_storage.insert(key, value);

            // Set the counter bytecode into the contract
            counter_genesis_storage.into_iter().for_each(|((_, k), v)| {
                let key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
                let value = Into::<StarkFelt>::into(v.0);
                counter_storage.insert(key, value);
            });

            // Deploy the contract account at genesis address
            let contract_account_class_hash = env.kakarot().contract_account_class_hash;
            let counter_address =
                StarknetContractAddress(Into::<StarkFelt>::into(counter_genesis_address).try_into().unwrap());
            let counter_storage_record = StorageRecord {
                nonce: Nonce(StarkFelt::from(0u8)),
                class_hash: ClassHash(contract_account_class_hash.into()),
                storage: counter_storage,
            };
            starknet.state.storage.insert(counter_address, counter_storage_record);
        })
        .await
        .unwrap();

        // Create a new counter contract pointing to the genesis initialized storage
        let counter_genesis = ContractAccount::new(counter_genesis_address, &starknet_client);
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
        let amount = U256::from_str("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let amount_split = split_u256_into_field_elements(amount);

        // This is equivalent to pre-funding the Starknet address with
        // 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb Fee Tokens.
        // The first storage key is for u256.low
        // The second storage key is for u256.high
        let expected_output = vec![
            (
                (
                    token_fee_address.into(),
                    get_storage_var_address(storage_variable_name, &[starknet_address]).unwrap().into(), /* offset for amount.low */
                ),
                amount_split[0].into(), // amount.low
            ),
            (
                (
                    token_fee_address.into(),
                    (get_storage_var_address(storage_variable_name, &[starknet_address]).unwrap()
                        + FieldElement::from(1u64))
                    .into(), // offset for amount.high
                ),
                amount_split[1].into(), // amount.high
            ),
        ];

        // When
        let result = genesis_fund_starknet_address(starknet_address, amount);

        // Then
        assert_eq!(result, expected_output);
    }

    /// This test verifies that the `genesis_set_storage_kakarot_contract_account` function
    /// generates the correct tuples for a given Starknet address, keys, storage value, and
    /// storage key offset.
    #[tokio::test]
    async fn test_genesis_set_storage_kakarot_contract_account() {
        // Given
        let starknet_address = FieldElement::from_hex_be("0x1234").unwrap();
        let key = U256::from_str("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let storage_variable_name = "storage_";
        let value = U256::from_str("0xccccccccccccccccccccccccccccccccdddddddddddddddddddddddddddddddd").unwrap();
        let value_split = split_u256_into_field_elements(value);

        // This is equivalent to setting the storage of Kakarot contract account's `storage_` variable at
        // index 0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb to
        // 0xccccccccccccccccccccccccccccccccdddddddddddddddddddddddddddddddd. The first storage key
        // is for value.low. The second storage key is for value.high.
        let expected_output = vec![
            (
                (
                    starknet_address.into(),
                    get_storage_var_address(storage_variable_name, &split_u256_into_field_elements(key))
                        .unwrap()
                        .into(), // offset for value.low
                ),
                value_split[0].into(), // value.low
            ),
            (
                (
                    starknet_address.into(),
                    (get_storage_var_address(storage_variable_name, &split_u256_into_field_elements(key)).unwrap()
                        + FieldElement::from(1u64))
                    .into(), // offset for value.high
                ),
                value_split[1].into(), // value.high
            ),
        ];

        // When
        let result = genesis_set_storage_kakarot_contract_account(starknet_address, key, value);
        // Then
        assert_eq!(result, expected_output);
    }

    #[tokio::test]
    async fn test_kakarot_contract_account_storage() {
        // Given
        let test_environment = Arc::new(KakarotTestEnvironment::new().await);

        // When
        // Use genesis_set_storage_kakarot_contract_account define the storage data
        // to be stored into the contract account
        let genesis_address = FieldElement::from_str("0x1234").unwrap();
        let expected_key =
            U256::from_str("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
        let expected_value =
            U256::from_str("0xccccccccccccccccccccccccccccccccdddddddddddddddddddddddddddddddd").unwrap();
        let genesis_storage_data =
            genesis_set_storage_kakarot_contract_account(genesis_address, expected_key, expected_value);

        // Create an atomic reference to the test environment to avoid dropping it
        let env = Arc::clone(&test_environment);
        // It is not possible to block the async test task, so we need to spawn a blocking task
        tokio::task::spawn_blocking(move || {
            // Get lock on the Starknet sequencer
            let mut starknet = env.sequencer().sequencer.starknet.blocking_write();
            let mut storage = HashMap::new();

            // Prepare the record to be inserted into the storage
            genesis_storage_data.into_iter().for_each(|((_, k), v)| {
                let storage_key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
                let storage_value = Into::<StarkFelt>::into(v.0);
                storage.insert(storage_key, storage_value);
            });

            // Set the storage record for the contract
            let contract_account_class_hash = env.kakarot().contract_account_class_hash;
            let genesis_address = StarknetContractAddress(Into::<StarkFelt>::into(genesis_address).try_into().unwrap());
            let storage_record = StorageRecord {
                nonce: Nonce(StarkFelt::from(0u8)),
                class_hash: ClassHash(contract_account_class_hash.into()),
                storage,
            };
            starknet.state.storage.insert(genesis_address, storage_record);
        })
        .await
        .unwrap();

        // Deploy the contract account with the set genesis storage and retrieve the storage on the contract
        let starknet_client = test_environment.client().starknet_provider();
        let genesis_contract = ContractAccount::new(genesis_address, &starknet_client);
        let [key_low, key_high] = split_u256_into_field_elements(expected_key);
        let actual_value =
            genesis_contract.storage(&key_low, &key_high, &StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Assert that the value stored in the contract is the same as the value we set in the genesis
        assert_eq!(expected_value, actual_value);
    }

    #[test]
    fn test_split_u256_into_field_elements() {
        let test_cases = vec![
            "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", // Normal case
            "0x0000000000000000000000000000000000000000000000000000000000000000", // Minimum value
            "0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff", // Maximum value
        ];

        test_cases.iter().for_each(|&value_str| {
            // Given
            // U256 value from the hexadecimal string
            let value = U256::from_str(value_str).unwrap();

            // When
            let result = split_u256_into_field_elements(value);

            // Then
            // Recalculate the U256 values using the resulting FieldElements
            // The first is the low 128 bits of the U256 value
            // The second is the high 128 bits of the U256 value and is left shifted by 128 bits
            let result: U256 =
                U256::from_be_bytes(result[1].to_bytes_be()) << 128 | U256::from_be_bytes(result[0].to_bytes_be());

            // Assert that the original and recombined U256 values are equal
            assert_eq!(result, value, "Failed for value: {value_str}");
        });
    }
}
