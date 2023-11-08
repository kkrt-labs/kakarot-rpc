#[cfg(test)]
mod tests {

    use std::sync::Arc;

    use kakarot_test_utils::deploy_helpers::{
        get_contract, get_contract_deployed_bytecode, KakarotTestEnvironmentContext,
    };
    use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
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
        let contract_account = ContractAccount::new(*ABDEL_STARKNET_ADDRESS, starknet_provider);

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
        let account = KakarotAccount::new(*ABDEL_STARKNET_ADDRESS, starknet_provider);

        // When
        let implementation = account.implementation(&BlockId::Tag(BlockTag::Latest)).await.unwrap();

        // Then
        assert_eq!(
            FieldElement::from_hex_be("0x4730612e9d26ebca8dd27be1af79cea613f7dee43f5b1584a172040e39f4063").unwrap(),
            implementation
        );
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_bytecode(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let contract_name = "Counter";
        let counter_starknet_address = kakarot_test_env_ctx.evm_contract(contract_name).addresses.starknet_address;
        let counter_contract = get_contract(contract_name);
        let expected_bytecode = get_contract_deployed_bytecode(counter_contract);

        let starknet_block_id = BlockId::Tag(BlockTag::Latest);
        let starknet_provider = kakarot_test_env_ctx.client().starknet_provider();
        let counter_contract_account = KakarotAccount::new(counter_starknet_address, starknet_provider);

        // When
        let actual_bytecode = counter_contract_account.bytecode(&starknet_block_id).await.unwrap();

        // Then
        assert_eq!(expected_bytecode.to_vec(), actual_bytecode.to_vec());
    }
}
