#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use alloy_consensus::{TxEip1559, EMPTY_ROOT_HASH};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Address, TxKind, B64, U256};
use alloy_rpc_types::Header;
use kakarot_rpc::{
    pool::mempool::maintain_transaction_pool,
    providers::eth_provider::{
        constant::U64_HEX_STRING_LEN,
        database::{
            filter::{self, format_hex, EthDatabaseFilterBuilder},
            types::header::StoredHeader,
        },
        error::SignatureError,
        ChainProvider,
    },
    test_utils::{
        eoa::Eoa,
        fixtures::{katana, katana_empty, setup},
        katana::Katana,
    },
};
use mongodb::{
    bson::doc,
    options::{UpdateModifications, UpdateOptions},
};
use reth_primitives::{sign_message, Transaction, TransactionSigned, TransactionSignedEcRecovered};
use reth_transaction_pool::{EthPooledTransaction, PoolTransaction, TransactionOrigin, TransactionPool};
use revm_primitives::B256;
use rstest::*;
use std::{sync::Arc, time::Duration};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transaction(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();

    // Create a sample transaction
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Check initial pool size
    assert_eq!(eth_client.mempool().pool_size().total, 0);

    // Add transaction to mempool
    let result = eth_client.mempool().add_transaction(TransactionOrigin::Local, transaction.clone()).await;

    // Ensure the transaction was added successfully
    assert!(result.is_ok());

    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending, queued and total transactions
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);

    // Get the EOA address
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    // get_transactions_by_sender_and_nonce test
    // Get transactions by sender address and nonce
    let sender_transaction = eth_client.mempool().get_transaction_by_sender_and_nonce(address, 0);
    // Check if the returned transaction hash matches
    assert_eq!(*sender_transaction.unwrap().hash(), transaction_signed.hash());

    // get_transactions_by_origin function test
    // Get transactions by origin
    let origin_transaction = eth_client.mempool().get_transactions_by_origin(TransactionOrigin::Local);
    // Check if the returned transaction hash matches
    assert_eq!(*origin_transaction[0].hash(), transaction_signed.hash());

    // get_local_transactions function test
    // Get local transactions
    let local_transaction = eth_client.mempool().get_local_transactions();
    // Check if the returned transaction hash matches
    assert_eq!(*local_transaction[0].hash(), transaction_signed.hash());
    assert_eq!(*local_transaction[0].hash(), *origin_transaction[0].hash());

    // all_transactions function tests
    // Get all transactions in the mempool
    let all_transactions = eth_client.mempool().all_transactions();
    // Check if the first pending transaction hash matches
    assert_eq!(*all_transactions.pending[0].hash(), transaction_signed.hash());
    // Ensure only one pending transaction is present
    assert_eq!(all_transactions.pending.len(), 1);
    // Ensure no queued transactions are present
    assert_eq!(all_transactions.queued.len(), 0);

    // remove_transactions function tests
    // Remove transaction by hash
    let _ = eth_client.mempool().remove_transactions(vec![transaction_signed.hash()]);
    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending, queued and total transactions after remove_transactions
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_external_transaction(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();

    // Create a sample transaction
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Add external transaction
    let result = eth_client.mempool().add_external_transaction(transaction).await;
    // Ensure the transaction was added successfully
    assert!(result.is_ok());

    // get_pooled_transaction_element function test
    // Get pooled transaction by hash
    let hashes = eth_client.mempool().get_pooled_transaction_element(transaction_signed.hash());
    // Check if the retrieved hash matches the expected hash
    assert_eq!(hashes.unwrap().hash(), &transaction_signed.hash());

    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending, queued and total transactions
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transactions(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();
    // Get the EOA address
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    // Set the number of transactions to create
    let transaction_number = 2;

    // Create multiple sample transactions
    let transactions =
        create_sample_transactions(&katana, transaction_number).await.expect("Failed to create sample transaction");

    // Collect pooled transactions
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();

    // Collect signed transactions
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    // Add transactions to mempool
    let _ = eth_client.mempool().add_transactions(TransactionOrigin::Local, pooled_transactions).await;

    // pending_transactions function tests
    // Get pending transactions
    let hashes = eth_client.mempool().pending_transactions();
    let expected_hashes = signed_transactions.iter().map(TransactionSigned::hash).collect::<Vec<_>>();
    let received_hashes = hashes.iter().map(|tx| *tx.hash()).collect::<Vec<_>>();
    assert_eq!(received_hashes, expected_hashes);

    // get_transactions_by_sender function tests
    // Get transactions by sender address
    let sender_transactions = eth_client.mempool().get_transactions_by_sender(address);
    let received_sender_transactions = sender_transactions.iter().map(|tx| *tx.hash()).collect::<Vec<_>>();
    assert_eq!(received_sender_transactions, expected_hashes);

    // unique_senders function test
    // Get unique senders from the mempool
    let unique_senders = eth_client.mempool().unique_senders();
    // Ensure the EOA address is in the unique senders
    assert!(unique_senders.contains(&address));

    // contains function test
    // Check if the first signed transaction is contained
    let contains = eth_client.mempool().contains(&signed_transactions[0].hash());
    assert!(contains);

    // mempool_size function tests
    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending transactions
    assert_eq!(mempool_size.pending, transaction_number);
    // Check queued transactions
    assert_eq!(mempool_size.queued, 0);
    // Check total transactions
    assert_eq!(mempool_size.total, transaction_number);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_external_transactions(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();

    // Create multiple sample transactions
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");

    // Collect pooled transactions
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();

    // Collect signed transactions
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    // Add external transactions to mempool
    let _ = eth_client.mempool().add_external_transactions(pooled_transactions).await;

    // pooled_transaction_hashes function tests
    // Get pooled transaction hashes
    let hashes = eth_client.mempool().pooled_transaction_hashes();
    // Check if the first signed transaction hash is present
    assert!(hashes.contains(&signed_transactions[0].hash()));
    // Check if the second signed transaction hash is present
    assert!(hashes.contains(&signed_transactions[1].hash()));
    // Ensure the hashes are not empty

    // pooled_transaction_hashes_max function test
    // Set maximum number of hashes to retrieve
    let hashes_max_number = 1;
    // Get pooled transaction hashes with a limit
    let hashes_max = eth_client.mempool().pooled_transaction_hashes_max(hashes_max_number);
    // Check if at least one signed transaction hash is present
    assert!(hashes_max.contains(&signed_transactions[0].hash()) || hashes_max.contains(&signed_transactions[1].hash()));
    // Ensure the number of hashes matches the limit
    assert_eq!(hashes_max.len(), hashes_max_number);
    // Ensure the hashes are not empty
    assert!(!hashes_max.is_empty());

    // get_external_transactions function test
    // Get external transactions
    let external_transactions = eth_client.mempool().get_external_transactions();

    // Check if the returned transactions match the expected ones, regardless of order
    assert_eq!(external_transactions.len(), 2);

    // Verify that both signed transactions are present in external transactions
    let external_hashes: Vec<_> = external_transactions.iter().map(|tx| *tx.hash()).collect();
    assert!(external_hashes.contains(&signed_transactions[0].hash()));
    assert!(external_hashes.contains(&signed_transactions[1].hash()));

    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending transactions
    assert_eq!(mempool_size.pending, 2);
    // Check queued transactions
    assert_eq!(mempool_size.queued, 0);
    // Check total transactions
    assert_eq!(mempool_size.total, 2);
    assert!(!hashes.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_transaction_event_listener(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();

    // Create a sample transaction
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Add transaction to mempool
    eth_client.mempool().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();

    // Get the transaction event listener
    let listener = eth_client.mempool().transaction_event_listener(transaction_signed.hash());
    // Ensure the listener exists
    assert!(listener.is_some());
    // Check if the listener's hash matches the transaction's hash
    assert_eq!(listener.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_private_transactions(#[future] katana_empty: Katana, _setup: ()) {
    let katana: Katana = katana_empty;

    let eth_client = katana.eth_client();

    // Create a sample transaction
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Add private transaction to mempool
    eth_client.mempool().add_transaction(TransactionOrigin::Private, transaction.clone()).await.unwrap();

    // Get private transactions
    let private_transaction = eth_client.mempool().get_private_transactions();
    // Check if the returned transaction hash matches
    assert_eq!(*private_transaction[0].hash(), transaction_signed.hash());
}

// Helper function to create a sample transaction
pub async fn create_sample_transactions(
    katana: &Katana,
    num_transactions: usize,
) -> Result<Vec<(EthPooledTransaction, TransactionSigned)>, SignatureError> {
    // Initialize a vector to hold transactions
    let mut transactions = Vec::new();
    // Get the Ethereum provider
    let eth_provider = katana.eth_provider();

    let signer = katana.eoa().evm_address().expect("Failed to get eoa address");

    // Get the chain ID
    let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

    for counter in 0..num_transactions {
        // Create a new EIP-1559 transaction
        let transaction = Transaction::Eip1559(TxEip1559 {
            chain_id,
            nonce: counter as u64,
            gas_limit: 21000,
            to: TxKind::Call(Address::random()),
            value: U256::from(1000),
            max_fee_per_gas: 875_000_000,
            max_priority_fee_per_gas: 0,
            ..Default::default()
        });

        // Sign the transaction
        let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();

        // Create a signed transaction
        let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);

        // Create an EC recovered signed transaction
        let transaction_signed_ec_recovered =
            TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer);

        // Get the encoded length of the transaction
        let encoded_length = transaction_signed_ec_recovered.clone().encode_2718_len();

        // Create a pooled transaction
        let eth_pooled_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);

        // Add the transaction to the vector
        transactions.push((eth_pooled_transaction, transaction_signed));
    }
    Ok(transactions)
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_maintain_mempool(#[future] katana: Katana, _setup: ()) {
    let eth_client = Arc::new(katana.eth_client());

    // Create two sample transactions at once
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transactions");

    // Extract and ensure we have two valid transactions from the transaction list.
    let ((transaction1, _), (transaction2, _)) = (
        transactions.first().expect("Expected at least one transaction").clone(),
        transactions.get(1).expect("Expected at least two transactions").clone(),
    );

    // Add transactions to the mempool
    eth_client.mempool().add_transaction(TransactionOrigin::Private, transaction1.clone()).await.unwrap();
    eth_client.mempool().add_transaction(TransactionOrigin::Private, transaction2.clone()).await.unwrap();

    // Start maintaining the transaction pool
    //
    // This task will periodically prune the mempool based on the given prune_duration.
    // For testing purposes, we set the prune_duration to 100 milliseconds.
    let prune_duration = Duration::from_millis(100);
    let eth_client_clone = Arc::clone(&eth_client);
    let maintain_task = tokio::spawn(async move {
        maintain_transaction_pool(eth_client_clone, prune_duration);
    });

    // Initialize the block number based on the current blockchain state from katana.
    let mut last_block_number = katana.block_number();

    // Loop to simulate new blocks being added to the blockchain every 100 milliseconds.
    for _ in 0..9 {
        // Sleep for 10 milliseconds to simulate the passage of time between blocks.
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Increment the block number to simulate the blockchain progressing.
        last_block_number += 1;

        // Format the block number in both padded and unpadded hexadecimal formats.
        let unpadded_block_number = format_hex(last_block_number, 0);
        let padded_block_number = format_hex(last_block_number, U64_HEX_STRING_LEN);

        // Get the block header collection from the database.
        let header_collection = eth_client.eth_provider().database().collection::<StoredHeader>();

        // Build a filter for updating the header based on the new block number.
        let filter = EthDatabaseFilterBuilder::<filter::Header>::default().with_block_number(last_block_number).build();

        // Insert a new header for the new block number in the database.
        eth_client
            .eth_provider()
            .database()
            .update_one(
                StoredHeader {
                    header: Header {
                        hash: B256::random(),
                        total_difficulty: Some(U256::default()),
                        mix_hash: Some(B256::default()),
                        nonce: Some(B64::default()),
                        withdrawals_root: Some(EMPTY_ROOT_HASH),
                        base_fee_per_gas: Some(0),
                        blob_gas_used: Some(0),
                        excess_blob_gas: Some(0),
                        number: last_block_number,
                        ..Default::default()
                    },
                },
                filter,
                true,
            )
            .await
            .expect("Failed to update header in database");

        // Update the header collection with the padded block number in the database.
        header_collection
            .update_one(
                doc! {"header.number": unpadded_block_number},
                UpdateModifications::Document(doc! {"$set": {"header.number": padded_block_number}}),
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .expect("Failed to update block number");

        // Check if both transactions are still in the mempool.
        // We expect them to still be in the mempool until 1 second has elapsed.
        assert!(eth_client.mempool().contains(transaction1.hash()), "Transaction 1 should still be in the mempool");
        assert!(eth_client.mempool().contains(transaction2.hash()), "Transaction 2 should still be in the mempool");
    }

    // Sleep for some additional time to allow the pruning to occur.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Verify that both transactions have been pruned from the mempool after the pruning duration.
    assert!(!eth_client.mempool().contains(transaction1.hash()), "Transaction 1 should be pruned after 1 second");
    assert!(!eth_client.mempool().contains(transaction2.hash()), "Transaction 2 should be pruned after 1 second");

    // Ensure the background task is stopped gracefully.
    maintain_task.abort();
}
