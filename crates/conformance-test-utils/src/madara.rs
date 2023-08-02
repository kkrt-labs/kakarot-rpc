use eyre::Result;
use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
use reth_primitives::Bytes;
use serde_json::Value;
use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;

pub const FEE_TOKEN_ADDRESS_HEX: &str = "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7";

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

/// Generates the genesis storage data for pre-funding a Starknet address on Madara.
///
/// This function calculates the storage key for the balance of the ERC20 Fee Token
/// contract using the provided Starknet address. The resulting JSON array represents the initial
/// storage of the Fee Token contract, where the account associated with the Starknet address is
/// pre-funded with the specified `amount`.
///
/// # Arguments
///
/// * `starknet_address` - The Starknet address to be funded
/// * `amount` - The amount of funds to allocate to the Starknet account.
///
/// # Returns
///
/// * `Result<Value>` - A JSON array representing the initial storage of the Fee Token contract
///   where the Starknet address has been pre-funded with the specified `amount`. Returns an error
///   if the constant `STARKNET_NATIVE_TOKEN` could not be converted to a `FieldElement` or if there
///   was an error calculating the storage variable address.
pub fn genesis_fund_starknet_address(starknet_address: FieldElement, amount: FieldElement) -> Result<Value> {
    // Compute the storage key for `ERC20_balances` and the Starknet address
    let storage_var_address = get_storage_var_address("ERC20_balances", &[starknet_address])?;

    let fee_token_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN)?;

    // Create the JSON array for the initial storage data
    let storage_data = serde_json::json!([[[Felt(fee_token_address), Felt(storage_var_address),], Felt(amount),]]);

    Ok(storage_data)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::str::FromStr;

    use kakarot_rpc_core::contracts::contract_account::ContractAccount;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
    use katana_core::backend::state::StorageRecord;
    use serde_json::json;
    use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};
    use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
    use starknet::providers::JsonRpcClient;
    use starknet_api::core::{ClassHash, ContractAddress as StarknetContractAddress, Nonce};
    use starknet_api::hash::StarkFelt;
    use starknet_api::state::StorageKey as StarknetStorageKey;

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

        // Assert that the expected and actual bytecodes are equal
        assert_eq!(deployed_bytecode, bytecode_actual);
    }

    /// This test verifies that the `genesis_fund_starknet_address` function generates the
    /// correct storage data for a given Starknet address and amount. The expected
    /// storage key was generated using the tested get_storage_var_address utility
    /// function.
    #[tokio::test]
    async fn test_genesis_fund_starknet_address() {
        // Given
        let starknet_address = FieldElement::from_hex_be("0x3").unwrap();
        let amount = FieldElement::from_hex_be("0xffffffffffffffffffffffffffffffff").unwrap();

        // This is the expected storage data for a given starknet_address and amount.
        let expected_output = json!({
            "storage": [
                [
                    [
                        "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                        "0x262e096a838c0d8f34f641ff917d47d7dcb345c69efe61d9ab6b675e7340fc6",
                    ],
                    "0xffffffffffffffffffffffffffffffff"
                ]
            ]
        });

        // When
        let result = genesis_fund_starknet_address(starknet_address, amount).unwrap();

        // Then
        assert_eq!(result, expected_output);
    }
}
