use std::str::FromStr;

use dojo_test_utils::rpc::MockJsonRpcTransport;
use reth_primitives::{BlockId, BlockNumberOrTag, Bytes, H256, U256, U64};
use reth_rpc_types::CallRequest;
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransactionV1};
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::sequencer::models::BlockId as SequencerBlockId;
use starknet::providers::{JsonRpcClient, SequencerGatewayProvider};
use starknet_crypto::FieldElement;

use super::config::{Network, SequencerGatewayProviderBuilder};
use crate::client::api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::config::StarknetConfig;
use crate::client::constants::{CHAIN_ID, COUNTER_ADDRESS_TESTNET1, INC_SELECTOR};
use crate::client::KakarotClient;
use crate::mock::constants::{
    ABDEL_ETHEREUM_ADDRESS, ABDEL_STARKNET_ADDRESS, ABDEL_STARKNET_ADDRESS_HEX, ACCOUNT_ADDRESS, ACCOUNT_ADDRESS_EVM,
    COUNTER_ADDRESS_EVM, INC_DATA, KAKAROT_ADDRESS, KAKAROT_TESTNET_ADDRESS, PROXY_ACCOUNT_CLASS_HASH,
    PROXY_ACCOUNT_CLASS_HASH_HEX,
};
use crate::mock::mock_starknet::{fixtures, mock_starknet_provider, AvailableFixtures, StarknetRpcFixture};
use crate::wrap_kakarot;

pub fn init_testnet_client() -> KakarotClient<SequencerGatewayProvider> {
    let kakarot_address = FieldElement::from_hex_be(KAKAROT_TESTNET_ADDRESS).unwrap();
    let config = StarknetConfig::new(Network::Goerli1Gateway, kakarot_address, Default::default());

    let provider = SequencerGatewayProviderBuilder::new(&Network::Goerli1Gateway).build();
    KakarotClient::new(config, provider)
}

pub fn init_mock_client(
    fixtures: Option<Vec<StarknetRpcFixture>>,
) -> KakarotClient<JsonRpcClient<MockJsonRpcTransport>> {
    let config = StarknetConfig::new(Network::Katana, *KAKAROT_ADDRESS, *PROXY_ACCOUNT_CLASS_HASH);
    let starknet_provider = mock_starknet_provider(fixtures);

    KakarotClient::new(config, starknet_provider)
}

#[tokio::test]
async fn test_block_number() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_mock_client(Some(fixtures));

    // When
    let block_number = client.block_number().await.unwrap();

    // Then
    assert_eq!(U64::from(19640), block_number);
}

#[tokio::test]
async fn test_nonce() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::GetNonce), AvailableFixtures::ComputeStarknetAddress]);
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
async fn test_transaction_by_hash() {
    // Given
    let fixtures = fixtures(vec![
        wrap_kakarot!(JsonRpcMethod::GetTransactionByHash),
        wrap_kakarot!(JsonRpcMethod::GetTransactionReceipt),
        AvailableFixtures::GetClassHashAt(ABDEL_STARKNET_ADDRESS_HEX.into(), PROXY_ACCOUNT_CLASS_HASH_HEX.into()),
        AvailableFixtures::GetEvmAddress,
    ]);
    let client = init_mock_client(Some(fixtures));

    // When
    let tx = match client
        .transaction_by_hash(
            H256::from_str("0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c").unwrap(),
        )
        .await
        .unwrap()
    {
        Some(tx) => tx,
        None => panic!("Tx should not be none"),
    };

    // Then
    assert_eq!(*ABDEL_ETHEREUM_ADDRESS, tx.from);
    assert_eq!(U256::from(0), tx.nonce);
}

#[tokio::test]
#[allow(deprecated)]
async fn test_simulate_transaction() {
    // Given
    let client = init_testnet_client();
    let block_id = SequencerBlockId::Latest;

    let block_number = client.block_number().await.unwrap().low_u64();
    let nonce = client.starknet_provider.get_nonce(*ACCOUNT_ADDRESS, block_id).await.unwrap();

    let calldata = vec![
        FieldElement::ONE,         // call array length
        *COUNTER_ADDRESS_TESTNET1, // counter address
        *INC_SELECTOR,             // selector
        FieldElement::ZERO,        // data offset length
        FieldElement::ZERO,        // data length
        FieldElement::ZERO,        // calldata length
    ];

    let tx = BroadcastedInvokeTransactionV1 {
        sender_address: *ACCOUNT_ADDRESS,
        calldata,
        max_fee: FieldElement::ZERO,
        nonce,
        signature: vec![],
    };

    // When
    let simulation = client.simulate_transaction(tx, block_number, true).await.unwrap();

    // Then
    assert!(simulation.fee_estimation.gas_price > 0);
    assert!(simulation.fee_estimation.gas_usage > 0);
    assert_eq!(
        simulation.fee_estimation.overall_fee,
        simulation.fee_estimation.gas_price * simulation.fee_estimation.gas_usage
    );
}

#[tokio::test]
async fn test_estimate_gas() {
    // Given
    let client = init_testnet_client();

    let request = CallRequest {
        from: Some(*ACCOUNT_ADDRESS_EVM),               // account address
        to: Some(*COUNTER_ADDRESS_EVM),                 // counter address
        data: Some(Bytes::from_str(INC_DATA).unwrap()), // call to inc()
        chain_id: Some(U64::from(CHAIN_ID)),            // "KKRT" chain id
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
