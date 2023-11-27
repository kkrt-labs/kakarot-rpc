use std::str::FromStr;

use reth_primitives::{BlockId, BlockNumberOrTag, Bytes, U256, U64};
use reth_rpc_types::{CallInput, CallRequest};
use starknet::providers::jsonrpc::JsonRpcMethod;

use crate::client::constants::CHAIN_ID;
use crate::mock::constants::{
    ABDEL_ETHEREUM_ADDRESS, ABDEL_STARKNET_ADDRESS, ACCOUNT_ADDRESS_EVM, COUNTER_ADDRESS_EVM, INC_DATA,
};
use crate::mock::mock_starknet::{fixtures, init_mock_client, init_testnet_client, AvailableFixtures};
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
    let evm_address = client.get_evm_address(&ABDEL_STARKNET_ADDRESS).await.unwrap();

    // Then
    assert_eq!(*ABDEL_ETHEREUM_ADDRESS, evm_address);
}

#[tokio::test]
async fn test_fee_history() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_mock_client(Some(fixtures));

    // When
    let count = 10;
    let block_count = U256::from(count);
    let newest_block = BlockNumberOrTag::Latest;
    let fee_history = client.fee_history(block_count, newest_block, None).await.unwrap();

    // Then
    assert_eq!(vec![U256::from(1); count + 1], fee_history.base_fee_per_gas);
    assert_eq!(vec![0.9; count], fee_history.gas_used_ratio);
    assert_eq!(U256::from(19630), fee_history.oldest_block);
    assert_eq!((Some(vec![vec![]])), fee_history.reward);
}

#[tokio::test]
async fn test_fee_history_should_return_oldest_block_0() {
    // Given
    let fixtures = fixtures(vec![]);
    let client = init_mock_client(Some(fixtures));

    // When
    let block_count = U256::from(10);
    let newest_block = BlockNumberOrTag::Number(1);
    let fee_history = client.fee_history(block_count, newest_block, None).await.unwrap();

    // Then
    assert_eq!(U256::from(0), fee_history.oldest_block);
}

#[tokio::test]
async fn test_estimate_gas() {
    // Given
    let client = init_testnet_client();

    let request = CallRequest {
        from: Some(*ACCOUNT_ADDRESS_EVM), // account address
        to: Some(*COUNTER_ADDRESS_EVM),   // counter address
        input: CallInput { input: None, data: Some(Bytes::from_str(INC_DATA).unwrap()) }, // call to inc()
        chain_id: Some(U64::from(CHAIN_ID)), // "KKRT" chain id
        ..Default::default()
    };
    let block_id = BlockId::Number(BlockNumberOrTag::Latest);

    // When
    let estimate = client.estimate_gas(request, block_id).await.unwrap();

    // Then
    assert!(estimate > U256::from(0));
}

#[tokio::test]
async fn test_gas_price() {
    // Given
    let client = init_testnet_client();

    // When
    let gas_price = client.gas_price().await.unwrap();

    // Then
    assert!(gas_price > U256::from(0));
}
