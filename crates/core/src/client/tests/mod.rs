use dojo_test_utils::rpc::MockJsonRpcTransport;
use reth_primitives::{BlockId, BlockNumberOrTag, U256, U64};
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::JsonRpcClient;

use crate::client::api::KakarotEthApi;
use crate::client::config::StarknetConfig;
use crate::client::KakarotClient;
use crate::mock::constants::{ABDEL_ADDRESS, KAKAROT_ADDRESS};
use crate::mock::mock_starknet::{fixtures, mock_starknet_provider, StarknetRpcFixture};

fn init_client(fixtures: Option<Vec<StarknetRpcFixture>>) -> KakarotClient<JsonRpcClient<MockJsonRpcTransport>> {
    let config = StarknetConfig { kakarot_address: *KAKAROT_ADDRESS, ..Default::default() };
    let provider = mock_starknet_provider(fixtures);

    KakarotClient::new(config, provider).unwrap()
}

#[tokio::test]
async fn test_block_number() {
    // Given
    let fixtures = fixtures(vec![JsonRpcMethod::BlockNumber]);
    let client = init_client(Some(fixtures));

    // When
    let block_number = client.block_number().await.unwrap();

    // Then
    assert_eq!(U64::from(19640), block_number);
}

#[tokio::test]
async fn test_nonce() {
    // Given
    let fixtures = fixtures(vec![JsonRpcMethod::GetNonce, JsonRpcMethod::Call]);
    let client = init_client(Some(fixtures));

    // When
    let nonce = client.nonce(*ABDEL_ADDRESS, BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

    // Then
    assert_eq!(U256::from(1), nonce);
}
