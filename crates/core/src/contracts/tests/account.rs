use rstest::*;
use starknet::core::types::{BlockId, BlockTag};

use crate::client::api::KakarotStarknetApi;
use crate::contracts::account::{Account, KakarotAccount};
use crate::test_utils::deploy_helpers::{get_contract, get_contract_deployed_bytecode, KakarotTestEnvironmentContext};
use crate::test_utils::fixtures::kakarot_test_env_ctx;

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
    let counter_contract_account = KakarotAccount::new(counter_starknet_address, starknet_provider.as_ref());

    // When
    let actual_bytecode = counter_contract_account.bytecode(&starknet_block_id).await.unwrap();

    // Then
    assert_eq!(expected_bytecode.to_vec(), actual_bytecode.to_vec());
}
