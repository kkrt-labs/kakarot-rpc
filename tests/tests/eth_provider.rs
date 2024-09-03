#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use alloy_primitives::{address, bytes};
use alloy_sol_types::{sol, SolCall};
use kakarot_rpc::{
    models::felt::Felt252Wrapper,
    providers::eth_provider::{
        constant::{MAX_LOGS, STARKNET_MODULUS},
        database::{ethereum::EthereumTransactionStore, types::transaction::StoredPendingTransaction},
        provider::EthereumProvider,
        BlockProvider, ChainProvider, GasProvider, LogProvider, ReceiptProvider, StateProvider, TransactionProvider,
    },
    test_utils::{
        eoa::Eoa,
        evm_contract::{EvmContract, KakarotEvmContract},
        fixtures::{contract_empty, counter, katana, plain_opcodes, setup},
        katana::Katana,
        tx_waiter::watch_tx,
    },
};
use reth_primitives::{
    sign_message, transaction::Signature, Address, BlockNumberOrTag, Bytes, Transaction, TransactionSigned, TxEip1559,
    TxKind, TxLegacy, B256, U256, U64,
};
use reth_rpc_types::{
    request::TransactionInput, serde_helpers::JsonStorageKey, state::AccountOverride, Filter, FilterBlockOption,
    FilterChanges, Log, RpcBlockHash, Topic, TransactionRequest,
};
use reth_transaction_pool::TransactionPool;
use rstest::*;
use starknet::core::types::{BlockTag, Felt};
use std::{collections::HashMap, sync::Arc};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let block_number = eth_provider.block_number().await.unwrap();

    // Then
    // Catch the most recent block number of the mocked Mongo Database
    let expected = U64::from(katana.block_number());
    assert_eq!(block_number, expected);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_chain_id(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default();

    // Then
    // ASCII code for "test" is 0x74657374
    // Since kaka_test > u32::MAX, we should return the last 4 bytes of the chain_id.
    assert_eq!(chain_id, U64::from(0x7465_7374_u64));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let block_hash = katana.most_recent_transaction().unwrap().block_hash.unwrap();

    // When
    let block = eth_provider.block_by_hash(block_hash, false).await.unwrap().unwrap();

    // Then
    assert_eq!(block.inner.header, katana.header_by_hash(block_hash).unwrap());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_by_number(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let block_number = katana.block_number();

    // When: Retrieving block by specific block number
    let block = eth_provider.block_by_number(BlockNumberOrTag::Number(block_number), false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the expected block number
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving earliest block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Earliest, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has block number zero
    assert_eq!(block.header.number, Some(0));

    // When: Retrieving latest block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Latest, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving finalized block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Finalized, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));

    // When: Retrieving safe block
    let block = eth_provider.block_by_number(BlockNumberOrTag::Safe, false).await.unwrap().unwrap();

    // Then: Ensure the retrieved block has the same block number as the most recent transaction
    assert_eq!(block.header.number, Some(block_number));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_transaction_count_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // Get the header of the first transaction
    let first_tx = katana.first_transaction().unwrap();
    let header = katana.header_by_hash(first_tx.block_hash.unwrap()).unwrap();

    // When
    let count = eth_provider.block_transaction_count_by_hash(header.hash.unwrap()).await.unwrap().unwrap();

    // Then
    assert_eq!(count, U256::from(1));

    // When
    let count = eth_provider
        .block_transaction_count_by_hash(katana.most_recent_transaction().unwrap().block_hash.unwrap())
        .await
        .unwrap()
        .unwrap();

    // Then
    assert_eq!(count, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_transaction_count_by_number(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance
    let eth_provider = katana.eth_provider();

    // Get the header of the first transaction
    let first_tx = katana.first_transaction().unwrap();
    let header = katana.header_by_hash(first_tx.block_hash.unwrap()).unwrap();

    // When: Retrieving transaction count for a specific block number
    let count = eth_provider
        .block_transaction_count_by_number(BlockNumberOrTag::Number(header.number.unwrap()))
        .await
        .unwrap()
        .unwrap();

    // Then: Ensure the retrieved transaction count matches the expected value
    assert_eq!(count, U256::from(1));

    // When: Retrieving transaction count for the block of the most recent transaction
    let block_number = katana.most_recent_transaction().unwrap().block_number.unwrap();
    let count =
        eth_provider.block_transaction_count_by_number(BlockNumberOrTag::Number(block_number)).await.unwrap().unwrap();

    // Then: Ensure the retrieved transaction count matches the expected value
    assert_eq!(count, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_balance(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    // When
    let eoa_balance = eth_provider.balance(eoa.evm_address().unwrap(), None).await.unwrap();

    // Then
    assert!(eoa_balance > U256::ZERO);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_storage_at(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    eoa.call_evm_contract(&counter, "inc", &[], 0).await.expect("Failed to increment counter");

    // Then
    let count = eth_provider.storage_at(counter_address, JsonStorageKey::from(U256::from(0)), None).await.unwrap();
    assert_eq!(count, B256::left_padding_from(&[0x1]));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_eoa(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let nonce = eth_provider.transaction_count(Address::ZERO, None).await.unwrap();

    // Then
    // Zero address shouldn't throw 'ContractNotFound', but return zero
    assert_eq!(U256::from(0), nonce);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_contract_account(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let nonce_initial = eth_provider.transaction_count(counter_address, None).await.unwrap();

    // Then
    assert_eq!(nonce_initial, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    let nonce_before = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();

    // When
    eoa.call_evm_contract(&counter, "inc", &[], 0).await.expect("Failed to increment counter");

    // Then
    let nonce_after = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();
    assert_eq!(nonce_before + U256::from(1), nonce_after);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let bytecode = eth_provider.get_code(counter_address, None).await.unwrap();

    // Then
    let counter_bytecode = <KakarotEvmContract as EvmContract>::load_contract_bytecode("Counter")
        .expect("Failed to load counter bytecode");
    let expected = counter_bytecode.deployed_bytecode.unwrap().0;
    assert_eq!(bytecode, expected);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_block_range(#[future] katana: Katana, _setup: ()) {
    // Given
    let provider = katana.eth_provider();

    // When
    let logs = provider.get_logs(Filter::default()).await.expect("Failed to get logs");

    // Then
    let FilterChanges::Logs(logs) = logs else { panic!("Expected logs") };
    assert!(!logs.is_empty());
}

/// Utility function to filter logs using the Ethereum provider.
/// Takes a filter and a provider, and returns the corresponding logs.
async fn filter_logs(filter: Filter, provider: Arc<dyn EthereumProvider>) -> Vec<Log> {
    // Call the provider to get logs using the filter.
    let logs = provider.get_logs(filter).await.expect("Failed to get logs");
    // If the result contains logs, return them, otherwise panic with an error.
    match logs {
        FilterChanges::Logs(logs) => logs,
        _ => panic!("Expected logs"),
    }
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_limit(#[future] katana: Katana, _setup: ()) {
    // Get the Ethereum provider from Katana.
    let provider = katana.eth_provider();

    // Set the limit of logs to be retrieved.
    std::env::set_var("MAX_LOGS", "500");

    // Add mock logs to the Katana instance's database.
    // The number of logs added is MAX_LOGS + 20, ensuring there are more logs than the limit.
    katana.add_mock_logs(((*MAX_LOGS).unwrap() + 20) as usize).await;

    // Assert that the number of logs returned by filter_logs is equal to the limit.
    // This ensures that the log retrieval respects the MAX_LOGS constraint.
    assert_eq!(filter_logs(Filter::default(), provider.clone()).await.len(), (*MAX_LOGS).unwrap() as usize);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_block_filter(#[future] katana: Katana, _setup: ()) {
    // Get the Ethereum provider from Katana.
    let provider = katana.eth_provider();

    // Get the first transaction from Katana.
    let first_transaction = katana.first_transaction().unwrap();
    let block_number = first_transaction.block_number.unwrap();
    let block_hash = first_transaction.block_hash.unwrap();

    // Get logs by block number from Katana.
    let logs_katana_block_number = katana.logs_by_block_number(block_number);
    // Get logs for a range of blocks from Katana.
    let logs_katana_block_range = katana.logs_by_block_range(0..u64::MAX / 2);
    // Get logs by block hash from Katana.
    let logs_katana_block_hash = katana.logs_by_block_hash(block_hash);
    // Get all logs from Katana.
    let all_logs_katana = katana.all_logs();

    // Verify logs filtered by block number.
    assert_eq!(filter_logs(Filter::default().select(block_number), provider.clone()).await, logs_katana_block_number);
    // Verify logs filtered by block hash.
    assert_eq!(filter_logs(Filter::default().select(block_hash), provider.clone()).await, logs_katana_block_hash);
    // Verify all logs.
    assert_eq!(filter_logs(Filter::default().select(0..), provider.clone()).await, all_logs_katana);
    // Verify logs filtered by a range of blocks.
    assert_eq!(filter_logs(Filter::default().select(0..u64::MAX / 2), provider.clone()).await, logs_katana_block_range);
    // Verify that filtering by an empty range returns an empty result.
    //
    // We skip the 0 block because we hardcoded it via our Mongo Fuzzer and so it can contain logs.
    assert!(filter_logs(Filter::default().select(1..1), provider.clone()).await.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_address_filter(#[future] katana: Katana, _setup: ()) {
    // Get the Ethereum provider from Katana.
    let provider = katana.eth_provider();

    // Get all logs from Katana.
    let all_logs_katana = katana.all_logs();

    // Get the first log address, or default address if logs are empty.
    let first_address = if all_logs_katana.is_empty() { Address::default() } else { all_logs_katana[0].address() };
    // Verify logs filtered by the first address.
    assert_eq!(
        filter_logs(Filter::new().address(vec![first_address]), provider.clone()).await,
        katana.logs_by_address(&[first_address])
    );

    // Create a vector to store a few addresses.
    let some_addresses: Vec<_> = all_logs_katana.iter().take(2).map(Log::address).collect();
    // Verify logs filtered by these few addresses.
    assert_eq!(
        filter_logs(Filter::new().address(some_addresses.clone()), provider.clone()).await,
        katana.logs_by_address(&some_addresses)
    );

    // Create a vector to store all addresses.
    let all_addresses: Vec<_> = all_logs_katana.iter().map(Log::address).collect();
    // Verify that all logs are retrieved when filtered by all addresses.
    assert_eq!(filter_logs(Filter::new().address(all_addresses), provider.clone()).await, all_logs_katana);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_topics(#[future] katana: Katana, _setup: ()) {
    // Given
    let provider = katana.eth_provider();
    let logs = katana.logs_with_min_topics(3);
    let topic_one = logs[0].topics()[0];
    let topic_two = logs[1].topics()[1];
    let topic_three = logs[0].topics()[2];
    let topic_four = logs[1].topics()[2];

    // Filter on the first topic
    let filter = Filter {
        topics: [topic_one.into(), Topic::default(), Topic::default(), Topic::default()],
        ..Default::default()
    };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 1);

    // Filter on the second topic
    let filter = Filter {
        topics: [Topic::default(), topic_two.into(), Topic::default(), Topic::default()],
        ..Default::default()
    };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 1);

    // Filter on the combination of topics three and four (should return 2 logs)
    let filter = Filter {
        topics: [Topic::default(), Topic::default(), vec![topic_three, topic_four].into(), Topic::default()],
        ..Default::default()
    };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 2);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_address(#[future] katana: Katana, _setup: ()) {
    // Given
    let provider = katana.eth_provider();
    let logs = katana.logs_with_min_topics(3);
    let address_one = logs[0].address();
    let address_two = logs[1].address();

    // Filter on the first address
    let filter = Filter { address: address_one.into(), ..Default::default() };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 1);

    // Filter on the second address
    let filter = Filter { address: address_two.into(), ..Default::default() };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 1);

    // Filter on the combination of both addresses
    let filter = Filter { address: vec![address_one, address_two].into(), ..Default::default() };
    assert_eq!(filter_logs(filter, provider.clone()).await.len(), 2);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_logs_block_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    let provider = katana.eth_provider();
    let logs = katana.logs_with_min_topics(0);
    let block_hash = logs[0].block_hash.unwrap();

    // Filter on block hash
    let filter = Filter { block_option: FilterBlockOption::AtBlockHash(block_hash), ..Default::default() };
    let filtered_logs = filter_logs(filter, provider.clone()).await;

    assert!(filtered_logs.iter().all(|log| log.block_hash.unwrap() == block_hash));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code_empty(#[future] contract_empty: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana: Katana = contract_empty.0;

    let counter = contract_empty.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();
    let counter_address = counter_address.try_into().expect("Failed to convert EVM address");

    // When
    let bytecode = eth_provider.get_code(counter_address, None).await.unwrap();

    // Then
    assert_eq!(bytecode, Bytes::default());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_code_no_contract(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let bytecode = eth_provider.get_code(Address::random(), None).await.unwrap();

    // Then
    assert_eq!(bytecode, Bytes::default());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_estimate_gas(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let eoa = counter.0.eoa();
    let eth_provider = counter.0.eth_provider();
    let counter = counter.1;

    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default();
    let counter_address: Felt252Wrapper = counter.evm_address.into();

    let request = TransactionRequest {
        from: Some(eoa.evm_address().unwrap()),
        to: Some(TxKind::Call(counter_address.try_into().unwrap())),
        input: TransactionInput { input: None, data: Some(bytes!("371303c0")) }, // selector of "function inc()"
        chain_id: Some(chain_id.to::<u64>()),
        ..Default::default()
    };

    // When
    let estimate = eth_provider.estimate_gas(request, None).await.unwrap();

    // Then
    assert!(estimate > U256::from(0));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_fee_history(#[future] katana: Katana, _setup: ()) {
    // Retrieve the Ethereum provider from the Katana instance.
    let eth_provider = katana.eth_provider();

    // Retrieve the most recent block number.
    let newest_block = katana.block_number();

    // To ensure that the range includes all mocked blocks.
    let block_count = u64::MAX;

    // Get the total number of blocks in the database.
    let nbr_blocks = katana.headers.len();

    // Call the fee_history method of the Ethereum provider.
    let fee_history =
        eth_provider.fee_history(U64::from(block_count), BlockNumberOrTag::Number(newest_block), None).await.unwrap();

    // Verify that the length of the base_fee_per_gas list in the fee history is equal
    // to the total number of blocks plus one.
    assert_eq!(fee_history.base_fee_per_gas.len(), nbr_blocks + 1);

    // Verify that the length of the gas_used_ratio list in the fee history is equal
    // to the total number of blocks.
    assert_eq!(fee_history.gas_used_ratio.len(), nbr_blocks);

    // Verify that the oldest block in the fee history is equal to zero.
    assert_eq!(fee_history.oldest_block, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[cfg(feature = "hive")]
async fn test_predeploy_eoa(#[future] katana: Katana, _setup: ()) {
    use alloy_primitives::b256;
    use futures::future::join_all;
    use kakarot_rpc::{providers::eth_provider::constant::hive::CHAIN_ID, test_utils::eoa::KakarotEOA};
    use starknet::providers::Provider;

    // Given
    let one_ether = 1_000_000_000_000_000_000u128;
    let one_tenth_ether = one_ether / 10;

    let eoa = katana.eoa();
    let eth_provider = katana.eth_provider();
    let starknet_provider = eth_provider.starknet_provider();
    let other_eoa_1 = KakarotEOA::new(
        b256!("00000000000000012330000000000000000000000000000000000000000abde1"),
        eth_provider.clone(),
    );
    let other_eoa_2 = KakarotEOA::new(
        b256!("00000000000000123123456000000000000000000000000000000000000abde2"),
        eth_provider.clone(),
    );
    let chain_id = starknet_provider.chain_id().await.unwrap();
    CHAIN_ID.set(chain_id).expect("Failed to set chain id");

    let evm_address = eoa.evm_address().unwrap();
    let balance_before = eth_provider.balance(eoa.evm_address().unwrap(), None).await.unwrap();
    eoa.transfer(other_eoa_1.evm_address().unwrap(), 2 * one_ether)
        .await
        .expect("Failed to transfer funds to other eoa 1");
    // Sleep for 2 seconds to let the transaction pass
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    eoa.transfer(other_eoa_2.evm_address().unwrap(), one_ether).await.expect("Failed to transfer funds to other eoa 2");
    // Sleep for 2 seconds to let the transaction pass
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // When
    let jh1 = tokio::task::spawn(async move {
        let _ = other_eoa_1
            .transfer(evm_address, 17 * one_tenth_ether)
            .await
            .expect("Failed to transfer funds back to eoa");
    });
    let jh2 = tokio::task::spawn(async move {
        let _ =
            other_eoa_2.transfer(evm_address, 3 * one_tenth_ether).await.expect("Failed to transfer funds back to eoa");
    });
    join_all([jh1, jh2]).await;

    // Then
    // Await all transactions to pass
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    let balance_after = eth_provider.balance(evm_address, None).await.unwrap();
    assert_eq!(balance_after, balance_before - U256::from(one_ether));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_receipts(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance and the most recent transaction
    let eth_provider = katana.eth_provider();
    let transaction = katana.most_recent_transaction().unwrap();

    // Then: Retrieve receipts by block number
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(
            transaction.block_number.unwrap(),
        ))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, transaction.transaction_index);
    assert_eq!(receipt.block_hash, transaction.block_hash);
    assert_eq!(receipt.block_number, transaction.block_number);

    // Then: Retrieve receipts by block hash
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Hash(RpcBlockHash::from(transaction.block_hash.unwrap()))))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(receipts.len(), 1);
    let receipt = receipts.first().unwrap();
    assert_eq!(receipt.transaction_index, transaction.transaction_index);
    assert_eq!(receipt.block_hash, transaction.block_hash);
    assert_eq!(receipt.block_number, transaction.block_number);

    // Then: Attempt to retrieve receipts for a non-existing block
    let receipts = eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Hash(RpcBlockHash::from(B256::from(U256::from(0x00c0_fefe))))))
        .await
        .unwrap();
    assert!(receipts.is_none());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_starknet_block_id(#[future] katana: Katana, _setup: ()) {
    // Given: Ethereum provider instance and the most recent transaction
    let eth_provider = katana.eth_provider();
    let transaction = katana.most_recent_transaction().unwrap();

    // When: Convert block number identifier to StarkNet block identifier
    let block_id = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(transaction.block_number.unwrap()));
    let pending_starknet_block_id = eth_provider.to_starknet_block_id(block_id).await.unwrap();

    // When: Convert block hash identifier to StarkNet block identifier
    let some_block_hash = reth_rpc_types::BlockId::Hash(RpcBlockHash::from(transaction.block_hash.unwrap()));
    let some_starknet_block_hash = eth_provider.to_starknet_block_id(some_block_hash).await.unwrap();

    // When: Convert block tag identifier to StarkNet block identifier
    let pending_block_tag = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Pending);
    let pending_block_tag_starknet = eth_provider.to_starknet_block_id(pending_block_tag).await.unwrap();

    // When: Attempt to convert an unknown block number identifier to StarkNet block identifier
    let unknown_block_number = reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(u64::MAX));
    let unknown_starknet_block_number = eth_provider.to_starknet_block_id(unknown_block_number).await;

    // Then: Ensure the converted StarkNet block identifiers match the expected values
    assert_eq!(pending_starknet_block_id, starknet::core::types::BlockId::Number(transaction.block_number.unwrap()));
    assert_eq!(
        some_starknet_block_hash,
        starknet::core::types::BlockId::Hash(Felt::from_bytes_be(
            &U256::from_be_slice(transaction.block_hash.unwrap().as_slice())
                .wrapping_rem(STARKNET_MODULUS)
                .to_be_bytes()
        ))
    );
    assert_eq!(pending_block_tag_starknet, starknet::core::types::BlockId::Tag(BlockTag::Pending));
    assert!(unknown_starknet_block_number.is_err());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Retrieve the current size of the mempool
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Retrieve the transaction from the database
    let tx: Option<StoredPendingTransaction> =
        eth_provider.database().get_first().await.expect("Failed to get transaction");

    // Assert that the number of retries is 0
    assert_eq!(0, tx.clone().unwrap().retries);

    let tx = tx.unwrap().tx;

    // Assert the transaction hash and block number
    assert_eq!(tx.hash, transaction_signed.hash());
    assert!(tx.block_number.is_none());

    // Retrieve the current size of the mempool
    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending transactions in the mempool is 1
    assert_eq!(mempool_size_after_send.pending, 1);
    assert_eq!(mempool_size_after_send.total, 1);
    let tx_in_mempool = eth_provider.mempool().unwrap().get(&tx.hash);
    // Assert that the transaction in the mempool exists
    assert!(tx_in_mempool.is_some());
    // Verify that the hash of the transaction in the mempool matches the expected hash
    assert_eq!(tx_in_mempool.unwrap().hash(), *tx.hash);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_wrong_nonce(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Retrieve the current size of the mempool
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Assert that the number of pending transactions in the mempool is 1
    assert_eq!(eth_provider.mempool().unwrap().pool_size().pending, 1);

    // Create a sample transaction with nonce 0 instead of 1
    let wrong_transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let wrong_signature = sign_message(katana.eoa().private_key(), wrong_transaction.signature_hash()).unwrap();
    let wrong_transaction_signed =
        TransactionSigned::from_transaction_and_signature(wrong_transaction, wrong_signature);

    // Retrieve the current size of the mempool
    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending transactions in the mempool is 1
    assert_eq!(mempool_size_after_send.pending, 1);
    assert_eq!(mempool_size_after_send.total, 1);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(wrong_transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Retrieve the current size of the mempool
    let mempool_size_after_wrong_send = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending transactions in the mempool is still 1 (wrong_transaction was not added to the mempool)
    assert_eq!(mempool_size_after_wrong_send.pending, 1);
    assert_eq!(mempool_size_after_wrong_send.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_exceed_size_limit(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::from(vec![0; 200 * 1024]),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Retrieve the current size of the mempool
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Retrieve the current size of the mempool
    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_exceed_max_priority_fee_per_gas(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: u128::MAX,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Retrieve the current size of the mempool
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Retrieve the current size of the mempool
    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_exceed_gas_limit(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: u64::MAX,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        input: Bytes::default(),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Retrieve the current size of the mempool
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Retrieve the current size of the mempool
    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_pre_eip_155(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let nonce: u64 = katana.eoa().nonce().await.unwrap().try_into().expect("Failed to convert nonce");

    // Use the transaction for the Arachnid deployer
    // https://github.com/Arachnid/deterministic-deployment-proxy
    let transaction = Transaction::Legacy(TxLegacy{value:U256::ZERO, chain_id: None, nonce, gas_price: 100_000_000_000, gas_limit: 100_000, to: TxKind::Create, input: bytes!("604580600e600039806000f350fe7fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe03601600081602082378035828234f58015156039578182fd5b8082525050506014600cf3") });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Set the WHITE_LISTED_EIP_155_TRANSACTION_HASHES env var to the hash
    // and add a blank space and an unknown hash to test the env var
    let hash = transaction_signed.hash();
    let random_hash = B256::random();
    std::env::set_var("WHITE_LISTED_EIP_155_TRANSACTION_HASHES", format!("{hash}, {random_hash}"));

    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Send the transaction
    let tx_hash = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    let bytes = tx_hash.0;
    let starknet_tx_hash = Felt::from_bytes_be(&bytes);

    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending transactions in the mempool is 1
    assert_eq!(mempool_size_after_send.pending, 1);
    assert_eq!(mempool_size_after_send.total, 1);

    watch_tx(eth_provider.starknet_provider(), starknet_tx_hash, std::time::Duration::from_millis(300), 60)
        .await
        .expect("Tx polling failed");

    // Then
    // Check that the Arachnid deployer contract was deployed
    let code = eth_provider.get_code(address!("5fbdb2315678afecb367f032d93f642f64180aa3"), None).await.unwrap();
    assert!(!code.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_wrong_signature(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id: 1,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        max_fee_per_gas: 875_000_000,
        ..Default::default()
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let mut transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Set an incorrect signature
    transaction_signed.signature = Signature::default();

    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Send the transaction
    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Retrieve the transaction from the database
    let tx: Option<StoredPendingTransaction> =
        eth_provider.database().get_first().await.expect("Failed to get transaction");

    // Assert that no transaction is found
    assert!(tx.is_none());

    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_wrong_chain_id(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let wrong_chain_id = 999; // An arbitrary wrong chain ID

    // Create a transaction with the wrong chain ID
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id: wrong_chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        input: Bytes::default(),
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Attempt to send the transaction
    let result = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    // Then
    assert!(result.is_err()); // Ensure the transaction is rejected

    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_insufficient_balance(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();
    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default().to();

    // Create a transaction with a value greater than the balance
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: 0,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::MAX,
        max_fee_per_gas: 875_000_000,
        max_priority_fee_per_gas: 0,
        input: Bytes::default(),
        access_list: Default::default(),
    });

    // Sign the transaction
    let signature = sign_message(eoa.private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    // Assert that the number of pending and total transactions in the mempool is 0
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.total, 0);

    // Attempt to send the transaction
    let _ = eth_provider.send_raw_transaction(transaction_signed.envelope_encoded()).await;

    let mempool_size_after_send = eth_provider.mempool().unwrap().pool_size();
    // Verify that the number of pending transactions in the mempool remains unchanged (0 tx)
    assert_eq!(mempool_size_after_send.pending, 0);
    assert_eq!(mempool_size_after_send.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_call_without_overrides(#[future] katana: Katana, _setup: ()) {
    // Obtain an Ethereum provider instance from the Katana instance
    let eth_provider = katana.eth_provider();

    // Get the EOA (Externally Owned Account) address from Katana
    let eoa_address = katana.eoa().evm_address().expect("Failed to get eoa address");

    // Create the first transaction request
    let request1 = TransactionRequest {
        from: Some(eoa_address),
        to: Some(TxKind::Call(Address::ZERO)),
        gas: Some(21000),
        gas_price: Some(10),
        value: Some(U256::from(1)),
        ..Default::default()
    };

    // Perform the first call with state override and high balance
    // The transaction should succeed
    let _ = eth_provider.call(request1, None, None, None).await.expect("Failed to call for a simple transfer");
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_call_with_state_override_balance_success(#[future] katana: Katana, _setup: ()) {
    // Obtain an Ethereum provider instance from the Katana instance
    let eth_provider = katana.eth_provider();

    // Generate an EOA address
    let eoa_address = address!("95222290DD7278Aa3Ddd389Cc1E1d165CC4BAfe5");

    // Create the second transaction request with a higher value
    let request = TransactionRequest {
        from: Some(eoa_address),
        to: Some(TxKind::Call(Address::ZERO)),
        gas: Some(21000),
        gas_price: Some(10),
        value: Some(U256::from(1_000_000)),
        ..Default::default()
    };

    // Initialize state override with the EOA address having a lower balance than the required value
    let mut state_override: HashMap<Address, AccountOverride> = HashMap::new();
    state_override
        .insert(eoa_address, AccountOverride { balance: Some(U256::from(1_000_000_000)), ..Default::default() });

    // Attempt to call and handle the result
    // Should succeed as the EOA balance is higher than the required value
    let _ = eth_provider
        .call(request, None, Some(state_override.clone()), None)
        .await
        .expect("Failed to call for a simple transfer");
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_call_with_state_override_balance_failure(#[future] katana: Katana, _setup: ()) {
    // Obtain an Ethereum provider instance from the Katana instance
    let eth_provider = katana.eth_provider();

    // Get the EOA (Externally Owned Account) address from Katana
    let eoa_address = katana.eoa().evm_address().expect("Failed to get eoa address");

    // Create the second transaction request with a higher value
    let request = TransactionRequest {
        from: Some(eoa_address),
        to: Some(TxKind::Call(Address::ZERO)),
        gas: Some(21000),
        gas_price: Some(10),
        value: Some(U256::from(1_000_000_001)),
        ..Default::default()
    };

    // Initialize state override with the EOA address having a lower balance than the required value
    let mut state_override: HashMap<Address, AccountOverride> = HashMap::new();
    state_override
        .insert(eoa_address, AccountOverride { balance: Some(U256::from(1_000_000_000)), ..Default::default() });

    // Attempt to call and handle the result
    let res = eth_provider.call(request, None, Some(state_override.clone()), None).await;

    // If the call succeeds, panic as an error was expected
    // If the call fails, get the error and convert it to a string
    let err = res.unwrap_err().to_string();

    // Check if the error is due to insufficient funds
    assert_eq!(err, "tracing error: transaction validation error: lack of funds (1000000000) for max fee (1000210001)");
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_call_with_state_override_bytecode(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    // Extract Katana instance from the plain_opcodes tuple
    let katana = plain_opcodes.0;

    // Obtain an Ethereum provider instance from the Katana instance
    let eth_provider = katana.eth_provider();

    // Convert KakarotEvmContract's EVM address to an Address type
    let contract_address = Address::from_slice(&plain_opcodes.1.evm_address.to_bytes_be()[12..]);

    // Get the EOA (Externally Owned Account) address from Katana
    let eoa_address = katana.eoa().evm_address().expect("Failed to get eoa address");

    // Define another Solidity contract with a different interface and bytecode
    sol! {
        #[sol(rpc, bytecode = "6080806040523460135760df908160198239f35b600080fdfe6080806040526004361015601257600080fd5b60003560e01c9081633fb5c1cb1460925781638381f58a146079575063d09de08a14603c57600080fd5b3460745760003660031901126074576000546000198114605e57600101600055005b634e487b7160e01b600052601160045260246000fd5b600080fd5b3460745760003660031901126074576020906000548152f35b34607457602036600319011260745760043560005500fea2646970667358221220e978270883b7baed10810c4079c941512e93a7ba1cd1108c781d4bc738d9090564736f6c634300081a0033")]
        #[derive(Debug)]
        contract Counter {
            uint256 public number;

            function setNumber(uint256 newNumber) public {
                number = newNumber;
            }

            function increment() public {
                number++;
            }
        }
    }

    // Extract the bytecode for the counter contract
    let bytecode = &Counter::BYTECODE[..];

    // Prepare the calldata for invoking the setNumber function
    let calldata = Counter::setNumberCall { newNumber: U256::from(10) }.abi_encode();

    // State override with the Counter bytecode
    let mut state_override: HashMap<Address, AccountOverride> = HashMap::new();
    state_override.insert(contract_address, AccountOverride { code: Some(bytecode.into()), ..Default::default() });

    // Define the transaction request for invoking the setNumber function
    let request = TransactionRequest {
        from: Some(eoa_address),
        to: Some(TxKind::Call(contract_address)),
        gas: Some(210_000),
        gas_price: Some(1_000_000_000_000_000_000_000),
        value: Some(U256::ZERO),
        nonce: Some(2),
        input: TransactionInput { input: Some(calldata.clone().into()), data: None },
        ..Default::default()
    };

    // Attempt to call the setNumber function and handle the result
    let _ = eth_provider
        .call(request, None, Some(state_override), None)
        .await
        .expect("Failed to set number in Counter contract");
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_transaction_by_hash(#[future] katana: Katana, _setup: ()) {
    // Given
    // Retrieve an instance of the Ethereum provider from the test environment
    let eth_provider = katana.eth_provider();
    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default().to();

    // Retrieve the first transaction from the test environment
    let first_transaction = katana.first_transaction().unwrap();

    // Check if the first transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(first_transaction.hash).await.unwrap().unwrap(), first_transaction);

    // Check if a non-existent transaction returns None
    assert!(eth_provider.transaction_by_hash(B256::random()).await.unwrap().is_none());

    // Generate a pending transaction to be stored in the pending transactions collection
    // Create a sample transaction
    let transaction = Transaction::Eip1559(TxEip1559 {
        chain_id,
        gas_limit: 21000,
        to: TxKind::Call(Address::random()),
        value: U256::from(1000),
        max_fee_per_gas: 875_000_000,
        ..Default::default()
    });

    // Sign the transaction
    let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
    let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

    // Send the transaction
    let _ = eth_provider
        .send_raw_transaction(transaction_signed.envelope_encoded())
        .await
        .expect("failed to send transaction");

    // Retrieve the pending transaction from the database
    let mut stored_transaction: StoredPendingTransaction =
        eth_provider.database().get_first().await.expect("Failed to get transaction").unwrap();

    let tx = stored_transaction.clone().tx;

    // Check if the pending transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(tx.hash).await.unwrap().unwrap(), tx);

    // Modify the block number of the pending transaction
    stored_transaction.tx.block_number = Some(1111);

    // Insert the transaction into the final transaction collection
    eth_provider.database().upsert_transaction(stored_transaction.into()).await.expect("Failed to insert documents");

    // Check if the final transaction is returned correctly by the `transaction_by_hash` method
    assert_eq!(eth_provider.transaction_by_hash(tx.hash).await.unwrap().unwrap().block_number, Some(1111));
}
