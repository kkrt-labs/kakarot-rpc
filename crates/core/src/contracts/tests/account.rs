#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use kakarot_rpc_core::client::api::KakarotStarknetApi;
    use kakarot_test_utils::execution::contract::KakarotEvmContract;
    use kakarot_test_utils::fixtures::counter;
    use kakarot_test_utils::sequencer::Katana;
    use reth_primitives::U256;
    use rstest::*;
    use starknet::core::types::{BlockId, BlockTag};
    use starknet_crypto::FieldElement;

    use crate::contracts::account::{Account, KakarotAccount};
    use crate::contracts::contract_account::ContractAccount;
    use crate::mock::constants::ABDEL_STARKNET_ADDRESS;
    use crate::mock::mock_starknet::{fixtures, mock_starknet_provider, AvailableFixtures};

    #[tokio::test]
    async fn test_nonce() {
        // Given
        let fixtures = fixtures(vec![AvailableFixtures::GetNonce]);
        let starknet_provider = Arc::new(mock_starknet_provider(Some(fixtures)));
        let contract_account = ContractAccount::new(*ABDEL_STARKNET_ADDRESS, &starknet_provider);

        // When
        let nonce = contract_account.nonce(&BlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Then
        assert_eq!(U256::from(1), nonce);
    }

    #[tokio::test]
    async fn test_implementation() {
        // Given
        let fixtures = fixtures(vec![AvailableFixtures::GetImplementation]);
        let starknet_provider = Arc::new(mock_starknet_provider(Some(fixtures)));
        let account = KakarotAccount::new(*ABDEL_STARKNET_ADDRESS, &starknet_provider);

        // When
        let implementation = account.implementation(&BlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Then
        assert_eq!(
            FieldElement::from_hex_be("0x4730612e9d26ebca8dd27be1af79cea613f7dee43f5b1584a172040e39f4063").unwrap(),
            implementation
        );
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bytecode(#[future] counter: (Katana, KakarotEvmContract)) {
        // Given
        let katana = counter.0;
        let counter = counter.1;
        let expected_bytecode =
            counter.bytecode.deployed_bytecode.expect("Missing deployed bytecode").bytecode.expect("Missing bytecode");
        let expected_bytecode = expected_bytecode.object.as_bytes().expect("Failed to convert bytecode to bytes");

        let starknet_block_id = BlockId::Tag(BlockTag::Latest);
        let starknet_provider = katana.client().starknet_provider();
        let counter_contract_account = KakarotAccount::new(counter.starknet_address, starknet_provider.as_ref());

        // When
        let actual_bytecode = counter_contract_account.bytecode(&starknet_block_id).await.unwrap();

        // Then
        assert_eq!(expected_bytecode.to_vec(), actual_bytecode.to_vec());
    }
}
