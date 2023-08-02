use ethers::types::Bytes;
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

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use kakarot_rpc_core::contracts::contract_account::ContractAccount;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{
        construct_kakarot_test_sequencer, deploy_kakarot_system, get_contract, get_contract_deployed_bytecode,
    };
    use katana_core::backend::state::StorageRecord;
    use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};
    use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
    use starknet::providers::JsonRpcClient;
    use starknet_api::core::{
        calculate_contract_address, ClassHash, ContractAddress as StarknetContractAddress, Nonce,
    };
    use starknet_api::hash::StarkFelt;
    use starknet_api::state::StorageKey as StarknetStorageKey;
    use starknet_api::transaction::{Calldata, ContractAddressSalt};

    use super::*;

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
        // Construct a Starknet test sequencer
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        // Define the expected funded amount for the Kakarot system
        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        // Deploy the Kakarot system
        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        // Get account proxy class hash and contract account class hash
        let contract_account_class_hash = deployed_kakarot.contract_account_class_hash;
        let account_proxy_class_hash = deployed_kakarot.proxy_class_hash;

        let actual_counter_address: FieldElement = (*calculate_contract_address(
            ContractAddressSalt(StarkFelt::from(1u8)),
            ClassHash(account_proxy_class_hash.into()),
            &Calldata(vec![].into()),
            StarknetContractAddress(StarkFelt::from(0u8).try_into().unwrap()),
        )
        .unwrap()
        .0
        .key())
        .into();

        // Get the bytecode for the deployed counter contract account
        let contract = get_contract("Counter");
        let deployed_counter_bytecode = get_contract_deployed_bytecode(contract);

        // Deploy a counter contract
        let (_, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "Counter",
                // no constructor is conveyed as a tuple
                (),
            )
            .await
            .unwrap();
        let expected_counter_address = deployed_addresses.starknet_address;

        // Create a new HTTP transport using the sequencer's URL
        let starknet_http_transport = StarknetHttpTransport::new(starknet_test_sequencer.url());

        // Create a new JSON RPC client using the HTTP transport
        let starknet_client = JsonRpcClient::new(starknet_http_transport);

        // Create a new counter contract linked to the expected deployed counter contract
        let expected_counter = ContractAccount::new(&starknet_client, expected_counter_address);

        // Create a new counter contract linked to the actual deployed counter contract
        let actual_counter = ContractAccount::new(&starknet_client, actual_counter_address);

        // Use genesis_load_bytecode to get the bytecode to be loaded into counter
        let counter_bytecode_storage = genesis_load_bytecode(&deployed_counter_bytecode, actual_counter_address);

        // It is not possible to block the async task, so we need to spawn a blocking task
        tokio::task::spawn_blocking(move || {
            // Get lock on the Starknet sequencer
            let mut starknet = starknet_test_sequencer.sequencer.starknet.blocking_write();

            // Deploy the proxy at the actual counter address, setting _implementation storage var to contract
            // account class hash
            let proxy_address =
                StarknetContractAddress(Into::<StarkFelt>::into(actual_counter_address).try_into().unwrap());
            let proxy_storage = StorageRecord {
                nonce: Nonce(StarkFelt::from(0u8)),
                class_hash: ClassHash(account_proxy_class_hash.into()),
                storage: HashMap::from([(
                    get_starknet_storage_key("_implementation", &[]),
                    contract_account_class_hash.into(),
                )]),
            };
            starknet.state.storage.insert(proxy_address, proxy_storage);

            let counter_addr: StarkFelt = actual_counter_address.into();

            // Get the counter storage
            let counter_storage = &mut starknet
                .state
                .storage
                .get_mut(&StarknetContractAddress(counter_addr.try_into().unwrap()))
                .unwrap()
                .storage;

            // Load the counter bytecode length into the contract
            let key = get_starknet_storage_key("bytecode_len_", &[]);
            let value = Into::<StarkFelt>::into(StarkFelt::from(deployed_counter_bytecode.len() as u64));
            counter_storage.insert(key, value);

            // Load the counter bytecode into the contract
            counter_bytecode_storage.into_iter().for_each(|((_, k), v)| {
                let key = StarknetStorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
                let value = Into::<StarkFelt>::into(v.0);
                counter_storage.insert(key, value);
            });
        })
        .await
        .unwrap();

        // Get the expected bytecode from the counter contract
        let bytecode_expected = expected_counter.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Get the actual bytecode from the counter contract
        let bytecode_actual = actual_counter.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Assert that the expected and actual bytecodes are equal
        assert_eq!(bytecode_expected, bytecode_actual);
    }
}
