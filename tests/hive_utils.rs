mod test_utils;

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::str::FromStr;

use ctor::ctor;
use kakarot_rpc::hive_utils::hive_genesis::serialize_hive_to_madara_genesis_config;
use kakarot_rpc::hive_utils::hive_genesis::GenesisLoader;
use kakarot_rpc::hive_utils::hive_genesis::HiveGenesisConfig;
use kakarot_rpc::hive_utils::kakarot::compute_starknet_address;
use kakarot_rpc::hive_utils::madara_utils::genesis_fund_starknet_address;
use kakarot_rpc::hive_utils::madara_utils::genesis_set_bytecode;
use kakarot_rpc::hive_utils::madara_utils::genesis_set_storage_kakarot_contract_account;
use kakarot_rpc::hive_utils::madara_utils::genesis_set_storage_starknet_contract;
use kakarot_rpc::hive_utils::types::ContractAddress;
use kakarot_rpc::hive_utils::types::StorageKey;
use kakarot_rpc::hive_utils::types::StorageValue;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::starknet_client::constants::STARKNET_NATIVE_TOKEN;
use kakarot_rpc::starknet_client::helpers::split_u256_into_field_elements;
use kakarot_rpc::starknet_client::ContractAccountReader;
use kakarot_rpc::starknet_client::Uint256 as CairoUint256;
use reth_primitives::Bytes;
use reth_primitives::U256;

use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;
use starknet_api::core::{ClassHash, ContractAddress as StarknetContractAddress, Nonce};
use starknet_api::hash::StarkFelt;
use starknet_api::state::StorageKey as StarknetStorageKey;
use test_utils::constants::ACCOUNT_ADDRESS;
use tracing_subscriber::{filter, FmtSubscriber};

use test_utils::evm_contract::KakarotEvmContract;
use test_utils::fixtures::{counter, katana};
use test_utils::sequencer::Katana;

use kakarot_rpc::starknet_client::KakarotClient;
use reth_primitives::Address;
use rstest::*;
use starknet::core::types::{BlockId, BlockTag};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

/// Kakarot Utils
/// This test is done against the Kakarot system deployed on the Starknet test sequencer.
/// It tests the compute_starknet_address function by comparing the result of the computation
/// with the result when called on the deployed Kakarot contract.
#[rstest]
#[tokio::test(flavor = "multi_thread")]
#[awt]
async fn test_compute_starknet_address(#[future] katana: Katana) {
    let client: &KakarotClient<JsonRpcClient<HttpTransport>> = katana.client();
    let kakarot_address = client.kakarot_address();
    let proxy_class_hash = client.proxy_account_class_hash();

    // Define the EVM address to be used for calculating the Starknet address
    let evm_address = Address::random();
    let evm_address_felt: Felt252Wrapper = evm_address.into();

    // Calculate the Starknet address
    let starknet_address = compute_starknet_address(kakarot_address, proxy_class_hash, evm_address_felt.into());

    // Calculate the expected Starknet address
    let expected_starknet_address =
        client.compute_starknet_address(&evm_address, &BlockId::Tag(BlockTag::Latest)).await.unwrap();

    // Assert that the calculated Starknet address matches the expected Starknet address
    assert_eq!(starknet_address, expected_starknet_address, "Starknet address does not match");
}

// Madara Utils Tests
#[ctor]
fn setup() {
    let filter = filter::EnvFilter::new("info");
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
}

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
    StarknetStorageKey(Into::<StarkFelt>::into(get_storage_var_address(var_name, args).unwrap()).try_into().unwrap())
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

#[ignore]
#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_counter_bytecode(#[future] counter: (Katana, KakarotEvmContract)) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let starknet_client = katana.client().starknet_provider();

    let counter_contract = ContractAccountReader::new(counter.evm_address, &starknet_client);

    // When
    let (deployed_evm_bytecode_len, deployed_evm_bytecode) = counter_contract.bytecode().call().await.unwrap();
    let deployed_evm_bytecode = Bytes::from(
        deployed_evm_bytecode.0.into_iter().filter_map(|x: FieldElement| u8::try_from(x).ok()).collect::<Vec<_>>(),
    );

    // Use genesis_set_bytecode to get the bytecode to be stored into counter
    let counter_genesis_address = FieldElement::from_str("0x1234").unwrap();
    let counter_genesis_storage = genesis_set_bytecode(&deployed_evm_bytecode, counter_genesis_address);

    // Get lock on the Starknet sequencer
    let mut starknet = katana.sequencer().sequencer.backend.state.write().await;
    let mut counter_storage = HashMap::new();

    // Set the counter bytecode length into the contract
    let key = get_starknet_storage_key("bytecode_len_", &[]);
    let value = StarkFelt::from(deployed_evm_bytecode_len);
    counter_storage.insert(key, value);

    // Set the counter bytecode into the contract
    counter_genesis_storage.into_iter().for_each(|((_, k), v)| {
        let key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
        let value = Into::<StarkFelt>::into(v.0);
        counter_storage.insert(key, value);
    });

    // Deploy the contract account at genesis address
    let contract_account_class_hash = katana.client().contract_account_class_hash();
    let counter_address = StarknetContractAddress(Into::<StarkFelt>::into(counter_genesis_address).try_into().unwrap());

    starknet.set_class_hash_at(counter_address, ClassHash(contract_account_class_hash.into())).unwrap();
    starknet.set_nonce(counter_address, Nonce(StarkFelt::from(1u8)));
    for (key, value) in counter_storage.into_iter() {
        starknet.set_storage_at(counter_address, key, value);
    }
    // Need to drop the lock on the sequencer to avoid deadlock, so we can then get the bytecode
    drop(starknet);

    // Create a new counter contract pointing to the genesis initialized storage
    let counter_genesis = ContractAccountReader::new(counter.evm_address, &starknet_client);
    let (_, genesis_evm_bytecode) = counter_genesis.bytecode().call().await.unwrap();
    let genesis_evm_bytecode = Bytes::from(
        genesis_evm_bytecode.0.into_iter().filter_map(|x: FieldElement| u8::try_from(x).ok()).collect::<Vec<_>>(),
    );

    // Then
    // Assert that the expected and actual bytecodes are equal
    assert_eq!(genesis_evm_bytecode, deployed_evm_bytecode);
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
                get_storage_var_address(storage_variable_name, &split_u256_into_field_elements(key)).unwrap().into(), // offset for value.low
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

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_kakarot_contract_account_storage(#[future] katana: Katana) {
    // When
    // Use genesis_set_storage_kakarot_contract_account define the storage data
    // to be stored into the contract account
    let genesis_address = FieldElement::from_str("0x1234").unwrap();
    let expected_key = U256::from_str("0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").unwrap();
    let expected_value = U256::from_str("0xccccccccccccccccccccccccccccccccdddddddddddddddddddddddddddddddd").unwrap();
    let genesis_storage_data =
        genesis_set_storage_kakarot_contract_account(genesis_address, expected_key, expected_value);

    // Get lock on the Starknet sequencer
    let mut starknet = katana.sequencer().sequencer.backend.state.write().await;
    let mut storage = HashMap::new();

    // Prepare the record to be inserted into the storage
    genesis_storage_data.into_iter().for_each(|((_, k), v)| {
        let storage_key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
        let storage_value = Into::<StarkFelt>::into(v.0);
        storage.insert(storage_key, storage_value);
    });

    // Set the storage record for the contract
    let contract_account_class_hash = katana.client().contract_account_class_hash();
    {
        let genesis_address = StarknetContractAddress(Into::<StarkFelt>::into(genesis_address).try_into().unwrap());

        starknet.set_class_hash_at(genesis_address, ClassHash(contract_account_class_hash.into())).unwrap();
        starknet.set_nonce(genesis_address, Nonce(StarkFelt::from(1u8)));
        for (key, value) in storage.into_iter() {
            starknet.set_storage_at(genesis_address, key, value);
        }
    }
    // Need to drop the lock on the sequencer to avoid deadlock, so we can then get the storage
    drop(starknet);

    // Deploy the contract account with the set genesis storage and retrieve the storage on the contract
    let starknet_client = katana.client().starknet_provider();
    let [key_low, key_high] = split_u256_into_field_elements(expected_key);
    let genesis_contract = ContractAccountReader::new(genesis_address, &starknet_client);

    // Convert a Uint256 to a Starknet storage key
    let storage_address = get_storage_var_address("storage_", &[key_low, key_high]).unwrap();

    let storage: CairoUint256 = genesis_contract.storage(&storage_address).call().await.unwrap();

    // TODO: replace by From<Uint256> for U256
    let low = storage.low;
    let high = storage.high;
    let actual_value =
        Into::<U256>::into(Felt252Wrapper::from(low)) + (Into::<U256>::into(Felt252Wrapper::from(high)) << 128);

    // Assert that the value stored in the contract is the same as the value we set in the genesis
    assert_eq!(expected_value, actual_value);
}

#[test]
fn test_read_hive_genesis() {
    // Read the hive genesis file
    let genesis = HiveGenesisConfig::from_file("src/hive_utils/test_data/hive_genesis.json").unwrap();

    // Verify the genesis file has the expected number of accounts
    assert_eq!(genesis.alloc.len(), 7);

    // Verify balance of each account is not empty
    assert!(genesis.alloc.values().all(|account_info| account_info.balance >= U256::from(0)));

    // Verify the storage field for each account
    // Since there is only one account with non-empty storage, we can hardcode the expected values
    assert!(genesis.alloc.values().all(|account_info| {
        account_info.storage.as_ref().map_or(true, |storage| {
            storage.len() == 2
                && *storage
                    .get(&U256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap())
                    .unwrap()
                    == U256::from_str("0x1234").unwrap()
                && *storage
                    .get(&U256::from_str("0x6661e9d6d8b923d5bbaab1b96e1dd51ff6ea2a93520fdc9eb75d059238b8c5e9").unwrap())
                    .unwrap()
                    == U256::from_str("0x01").unwrap()
        })
    }));

    // Verify the code field for each account, if exists, is not empty
    assert!(genesis
        .alloc
        .values()
        .all(|account_info| account_info.code.as_ref().map_or(true, |code| !code.is_empty())));
}

#[tokio::test]
async fn test_madara_genesis() {
    // Given
    let hive_genesis = HiveGenesisConfig::from_file("src/hive_utils/test_data/hive_genesis.json").unwrap();
    let madara_loader =
        serde_json::from_str::<GenesisLoader>(std::include_str!("../src/hive_utils/test_data/madara_genesis.json"))
            .unwrap();
    let combined_genesis_path = Path::new("src/hive_utils/test_data/combined_genesis.json");
    let compiled_path = Path::new("./cairo-contracts/build");

    // When
    serialize_hive_to_madara_genesis_config(hive_genesis, madara_loader, combined_genesis_path, compiled_path)
        .await
        .unwrap();

    let combined_genesis_file = File::open(combined_genesis_path)
        .unwrap_or_else(|_| panic!("Failed to open file at path {:?}", &combined_genesis_path));

    // Then
    let loader: GenesisLoader = serde_json::from_reader(&combined_genesis_file)
        .unwrap_or_else(|_| panic!("Failed to read from file at path {:?}", &combined_genesis_path));
    assert_eq!(9 + 2 + 7, loader.contracts.len()); // 9 original + 2 Kakarot contracts + 7 hive

    // After
    std::fs::remove_file(combined_genesis_path)
        .unwrap_or_else(|_| panic!("Failed to remove file at path {:?}", combined_genesis_path));
}
