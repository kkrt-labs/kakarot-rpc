use std::str::FromStr;

use reth_primitives::{BlockId, BlockNumberOrTag, Bytes, H256, U256, U64};
use reth_rpc_types::{CallInput, CallRequest, Filter, FilterBlockOption, FilterChanges, Log, ValueOrArray};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction};
use starknet::providers::jsonrpc::JsonRpcMethod;
use starknet::providers::sequencer::models::BlockId as SequencerBlockId;
use starknet_crypto::FieldElement;

use crate::client::api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::constants::{CHAIN_ID, COUNTER_ADDRESS_TESTNET1, INC_SELECTOR};
use crate::mock::constants::{
    ABDEL_ETHEREUM_ADDRESS, ABDEL_STARKNET_ADDRESS, ABDEL_STARKNET_ADDRESS_HEX, ACCOUNT_ADDRESS, ACCOUNT_ADDRESS_EVM,
    COUNTER_ADDRESS_EVM, INC_DATA, PROXY_ACCOUNT_CLASS_HASH_HEX,
};
use crate::mock::mock_starknet::{fixtures, init_mock_client, init_testnet_client, AvailableFixtures};
use crate::wrap_kakarot;

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
            H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9").unwrap(),
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

    let tx = BroadcastedInvokeTransaction {
        sender_address: *ACCOUNT_ADDRESS,
        calldata,
        max_fee: FieldElement::ZERO,
        nonce,
        signature: vec![],
        is_query: true,
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

#[tokio::test]
async fn test_get_logs_from_bigger_than_current() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_mock_client(Some(fixtures));
    let filter = Filter {
        block_option: FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(19641)),
            to_block: Some(BlockNumberOrTag::Number(19642)),
        },
        ..Default::default()
    };

    // When
    let logs = client.get_logs(filter).await.unwrap();

    // Then
    assert_eq!(logs, FilterChanges::Empty);
}

#[tokio::test]
async fn test_get_logs_to_less_than_from() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber)]);
    let client = init_mock_client(Some(fixtures));
    let filter = Filter {
        block_option: FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(2)),
            to_block: Some(BlockNumberOrTag::Number(1)),
        },
        ..Default::default()
    };

    // When
    let logs = client.get_logs(filter).await.unwrap();

    // Then
    assert_eq!(logs, FilterChanges::Empty);
}

#[tokio::test]
async fn test_get_logs() {
    // Given
    let fixtures = fixtures(vec![wrap_kakarot!(JsonRpcMethod::BlockNumber), wrap_kakarot!(JsonRpcMethod::GetEvents)]);
    let client = init_mock_client(Some(fixtures));
    let filter = Filter {
        block_option: FilterBlockOption::Range {
            from_block: Some(BlockNumberOrTag::Number(0)),
            to_block: Some(BlockNumberOrTag::Number(10)),
        },
        address: ValueOrArray::Value(*ABDEL_ETHEREUM_ADDRESS).into(),
        ..Default::default()
    };

    // When
    let logs = client.get_logs(filter).await.unwrap();

    // Then
    match logs {
        FilterChanges::Logs(logs) => {
            assert_eq!(2, logs.len());
            assert_eq!(
                Log {
                    address: *ABDEL_ETHEREUM_ADDRESS,
                    topics: vec![],
                    data: Bytes::from_str("0xdead").unwrap(),
                    block_hash: Some(
                        H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e").unwrap()
                    ),
                    block_number: Some(U256::from(5u8)),
                    transaction_hash: Some(
                        H256::from_str("0x032e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258").unwrap()
                    ),
                    transaction_index: None,
                    log_index: None,
                    removed: false
                },
                logs[0]
            );
            assert_eq!(
                Log {
                    address: *ABDEL_ETHEREUM_ADDRESS,
                    topics: vec![],
                    data: Bytes::from_str("0xbeef").unwrap(),
                    block_hash: Some(
                        H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e").unwrap()
                    ),
                    block_number: Some(U256::from(5u8)),
                    transaction_hash: Some(
                        H256::from_str("0x01b7ec62724de1faba75fdc75cf11c1f855af33e4fe5f36d8a201237f3c9f257").unwrap()
                    ),
                    transaction_index: None,
                    log_index: None,
                    removed: false
                },
                logs[1]
            )
        }
        _ => panic!("Expected FilterChanges::Logs variant, got {:?}", logs),
    }
}
