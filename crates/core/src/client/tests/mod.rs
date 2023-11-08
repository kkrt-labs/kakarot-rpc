use reth_primitives::{BlockId, BlockNumberOrTag, U256};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};
use starknet::providers::jsonrpc::JsonRpcMethod;

use crate::mock::constants::{ABDEL_ETHEREUM_ADDRESS, ABDEL_STARKNET_ADDRESS};
use crate::mock::mock_starknet::{fixtures, init_mock_client, AvailableFixtures};
use crate::wrap_kakarot;

#[tokio::test]
async fn test_nonce() {
    // Given
    let fixtures = fixtures(vec![
        wrap_kakarot!(JsonRpcMethod::GetNonce),
        AvailableFixtures::ComputeStarknetAddress,
        AvailableFixtures::GetImplementation,
    ]);
    let client = init_mock_client(Some(fixtures));

    // When
    let nonce = client.nonce(*ABDEL_ETHEREUM_ADDRESS, BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();

    // Then
    assert_eq!(U256::from(1), nonce);
}

#[tokio::test]
async fn test_get_evm_address() {
    // Given
    let fixtures = fixtures(vec![AvailableFixtures::GetEvmAddress]);
    let client = init_mock_client(Some(fixtures));

    // When
    let evm_address =
        client.get_evm_address(&ABDEL_STARKNET_ADDRESS, &StarknetBlockId::Tag(BlockTag::Latest)).await.unwrap();

    // Then
    assert_eq!(*ABDEL_ETHEREUM_ADDRESS, evm_address);
}
