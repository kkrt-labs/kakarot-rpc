use dojo_test_utils::rpc::MockJsonRpcTransport;
use reth_primitives::{BlockId, BlockNumberOrTag, U256, U64};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::JsonRpcClient;

use crate::client::api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::config::StarknetConfig;
use crate::client::KakarotClient;
use crate::mock::constants::{
    ABDEL_ETHEREUM_ADDRESS, ABDEL_STARKNET_ADDRESS, KAKAROT_ADDRESS, PROXY_ACCOUNT_CLASS_HASH,
};
use crate::mock::mock_starknet::{fixtures, mock_starknet_provider, AvailableFixtures, StarknetRpcFixture};
use crate::wrap_kakarot;

pub fn init_client(fixtures: Option<Vec<StarknetRpcFixture>>) -> KakarotClient<JsonRpcClient<MockJsonRpcTransport>> {
    let config = StarknetConfig {
        kakarot_address: *KAKAROT_ADDRESS,
        proxy_account_class_hash: *PROXY_ACCOUNT_CLASS_HASH,
        ..Default::default()
    };
    let provider = mock_starknet_provider(fixtures);

    KakarotClient::new(config, provider)
}

#[tokio::test]
async fn test_block_number() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_client(Some(fixtures));

    // When
    let block_number = client.block_number().await.unwrap();

    // Then
    assert_eq!(U64::from(19640), block_number);
}

#[tokio::test]
async fn test_nonce() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::GetNonce), AvailableFixtures::ComputeStarknetAddress]);
    let client = init_client(Some(fixtures));

    // When
    let nonce = client.nonce(*ABDEL_ETHEREUM_ADDRESS, BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();

    // Then
    assert_eq!(U256::from(1), nonce);
}

#[tokio::test]
async fn test_get_evm_address() {
    // Given
    let fixtures = fixtures(vec![AvailableFixtures::GetEvmAddress]);
    let client = init_client(Some(fixtures));

    // When
    let evm_address =
        client.get_evm_address(&ABDEL_STARKNET_ADDRESS, &StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

    // Then
    assert_eq!(*ABDEL_ETHEREUM_ADDRESS, evm_address);
}

#[tokio::test]
async fn test_fee_history() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_client(Some(fixtures));

    // When
    let count = 10;
    let block_count = U256::from(count);
    let newest_block = BlockNumberOrTag::Latest;
    let fee_history = client.fee_history(block_count, newest_block, None).await.unwrap();

    // Then
    assert_eq!(vec![U256::from(1); count + 1], fee_history.base_fee_per_gas);
    assert_eq!(vec![0.9; count], fee_history.gas_used_ratio);
    assert_eq!(U256::from(19630), fee_history.oldest_block);
    assert_eq!(None, fee_history.reward);
}

#[tokio::test]
async fn test_fee_history_should_return_oldest_block_0() {
    // Given
    let fixtures = fixtures(vec![]);
    let client = init_client(Some(fixtures));

    // When
    let block_count = U256::from(10);
    let newest_block = BlockNumberOrTag::Number(1);
    let fee_history = client.fee_history(block_count, newest_block, None).await.unwrap();

    // Then
    assert_eq!(U256::from(0), fee_history.oldest_block);
}
