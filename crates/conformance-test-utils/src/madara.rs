use reth_primitives::Bytes;
use starknet::core::types::FieldElement;
use starknet::core::utils::get_storage_var_address;

use crate::types::{ContractAddress, Felt, StorageKey, StorageValue};

pub fn genesis_load_bytecode(
    bytecode: Bytes,
    address: FieldElement,
) -> Vec<((ContractAddress, StorageKey), StorageValue)> {
    bytecode
        .chunks(16)
        .enumerate()
        .map(|(i, x)| {
            let mut storage_value = [0u8; 32];
            storage_value[32 - x.len()..].copy_from_slice(x);
            let storage_value = FieldElement::from_bytes_be(&storage_value).unwrap().into(); //safe unwrap since x.len() == 16

            let storage_key: Felt = get_storage_var_address("bytecode_", &[FieldElement::from(i)]).unwrap().into(); // safe unwrap since bytecode_ is all ascii

            ((address.into(), storage_key), storage_value)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use kakarot_rpc_core::contracts::contract_account::ContractAccount;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
    use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};
    use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
    use starknet::providers::JsonRpcClient;
    use starknet_api::core::ContractAddress as StarknetContractAddress;
    use starknet_api::hash::StarkFelt;
    use starknet_api::state::StorageKey;

    const TEST_BYTECODE: &str = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
    const BIG_ENDIAN_BYTECODE: &str = "0x1234567890abcdef1234567890abcdef";

    const COUNTER_BYTECODE: &str = "0x608060405234801561001057600080fd5b50600436106100625760003560e01c806306661abd14610067578063371303c0146100825780637c507cbd1461008c578063b3bcfa8214610094578063d826f88f1461009c578063f0707ea9146100a5575b600080fd5b61007060005481565b60405190815260200160405180910390f35b61008a6100ad565b005b61008a6100c6565b61008a610106565b61008a60008055565b61008a610139565b60016000808282546100bf919061017c565b9091555050565b60008054116100f05760405162461bcd60e51b81526004016100e790610195565b60405180910390fd5b6000805490806100ff836101dc565b9190505550565b60008054116101275760405162461bcd60e51b81526004016100e790610195565b60016000808282546100bf91906101f3565b600080541161015a5760405162461bcd60e51b81526004016100e790610195565b60008054600019019055565b634e487b7160e01b600052601160045260246000fd5b8082018082111561018f5761018f610166565b92915050565b60208082526027908201527f636f756e742073686f756c64206265207374726963746c7920677265617465726040820152660207468616e20360cc1b606082015260800190565b6000816101eb576101eb610166565b506000190190565b8181038181111561018f5761018f61016656fea26469706673582212203091d34e6cbebc53198d4c0d09786b51423a7ae0de314456c74c68aaccc311e364736f6c63430008110033";

    use super::*;

    #[test]
    fn test_genesis_load_bytecode() {
        // Given
        let bytecode = Bytes::from_str(TEST_BYTECODE).unwrap();
        let address = *ACCOUNT_ADDRESS;

        // When
        let storage = genesis_load_bytecode(bytecode, address);

        // Then
        let expected_storage: Vec<((Felt, Felt), Felt)> = vec![
            (
                (address.into(), get_storage_var_address("bytecode_", &[FieldElement::from(0u8)]).unwrap().into()),
                FieldElement::from_hex_be(BIG_ENDIAN_BYTECODE).unwrap().into(),
            ),
            (
                (address.into(), get_storage_var_address("bytecode_", &[FieldElement::from(1u8)]).unwrap().into()),
                FieldElement::from_hex_be(BIG_ENDIAN_BYTECODE).unwrap().into(),
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

        // Deploy a counter contract which
        let (_, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "Counter",
                // no constructor is conveyed as a tuple
                (),
            )
            .await
            .unwrap();
        let expected_counter_address = deployed_addresses[1]; // starknet address

        // Deploy a counter contract
        let (_, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "Safe",
                // no constructor is conveyed as a tuple
                (),
            )
            .await
            .unwrap();
        let actual_counter_address = deployed_addresses[1];

        // Create a new HTTP transport using the sequencer's URL
        let starknet_http_transport = StarknetHttpTransport::new(starknet_test_sequencer.url());

        // Create a new JSON RPC client using the HTTP transport
        let starknet_client = JsonRpcClient::new(starknet_http_transport);

        // Create a new counter contract linked to the expected deployed counter contract
        let expected_counter = ContractAccount::new(&starknet_client, expected_counter_address);

        // Create a new counter contract linked to the actual deployed counter contract
        let actual_counter = ContractAccount::new(&starknet_client, actual_counter_address);

        // Use genesis_load_bytecode to get the bytecode to be loaded into counter
        let counter_bytecode_storage =
            genesis_load_bytecode(Bytes::from_str(COUNTER_BYTECODE).unwrap(), actual_counter_address);

        // It is not possible to block the async task, so we need to spawn a blocking task
        tokio::task::spawn_blocking(move || {
            // Get the counter storage
            let mut starknet_wrapper = starknet_test_sequencer.sequencer.starknet.blocking_write();
            let addr: StarkFelt = actual_counter_address.into();
            let counter_storage = &mut starknet_wrapper
                .state
                .storage
                .get_mut(&StarknetContractAddress(addr.try_into().unwrap()))
                .unwrap()
                .storage;

            // Load the counter bytecode length into the caller contract
            let key = StorageKey(
                Into::<StarkFelt>::into(get_storage_var_address("bytecode_len_", &[]).unwrap()).try_into().unwrap(),
            );
            let value = Into::<StarkFelt>::into(StarkFelt::from(counter_bytecode_storage.len() as u32));
            println!("key: {:?}, value: {:?}", key, value);
            counter_storage.insert(key, value);

            // Load the counter bytecode into the caller contract
            counter_bytecode_storage.into_iter().for_each(|((_, k), v)| {
                let key = StorageKey(Into::<StarkFelt>::into(k.0).try_into().unwrap());
                let value = Into::<StarkFelt>::into(v.0);
                println!("key: {:?}, value: {:?}", key, value);
                counter_storage.insert(key, value);
            });

            starknet_wrapper.generate_latest_block();
        })
        .await
        .unwrap();

        // Get the expected bytecode from the counter contract
        let bytecode_expected = expected_counter.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Get the actual bytecode from the counter contract
        let bytecode_actual = actual_counter.bytecode(&StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Get the actual bytecode loaded into the counter contract
        assert_eq!(bytecode_expected, bytecode_actual);
    }
}
