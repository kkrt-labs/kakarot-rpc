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
use crate::mock::mock_starknet::{fixtures, mock_starknet_provider, KakarotJsonRpcMethod, StarknetRpcFixture};
use crate::wrap_kakarot;

pub fn init_client(fixtures: Option<Vec<StarknetRpcFixture>>) -> KakarotClient<JsonRpcClient<MockJsonRpcTransport>> {
    let config = StarknetConfig {
        kakarot_address: *KAKAROT_ADDRESS,
        proxy_account_class_hash: *PROXY_ACCOUNT_CLASS_HASH,
        ..Default::default()
    };
    let provider = mock_starknet_provider(fixtures);

    KakarotClient::new(config, provider).unwrap()
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
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::GetNonce), KakarotJsonRpcMethod::ComputeStarknetAddress]);
    let client = init_client(Some(fixtures));

    // When
    let nonce = client.nonce(*ABDEL_ETHEREUM_ADDRESS, BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();

    // Then
    assert_eq!(U256::from(1), nonce);
}

#[tokio::test]
async fn test_get_evm_address() {
    // Given
    let fixtures = fixtures(vec![KakarotJsonRpcMethod::GetEvmAddress]);
    let client = init_client(Some(fixtures));

    // When
    let evm_address =
        client.get_evm_address(&ABDEL_STARKNET_ADDRESS, &StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

    // Then
    assert_eq!(*ABDEL_ETHEREUM_ADDRESS, evm_address);
}
