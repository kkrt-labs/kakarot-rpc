#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use kakarot_rpc::{
    providers::eth_provider::{error::SignatureError, ChainProvider},
    test_utils::{
        eoa::Eoa,
        fixtures::{katana, setup},
        katana::Katana,
    },
};
use reth_primitives::{
    sign_message, Address, Transaction, TransactionSigned, TransactionSignedEcRecovered, TxEip1559, TxKind, U256,
};
use reth_transaction_pool::{EthPooledTransaction, TransactionOrigin, TransactionPool};
use rstest::*;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transaction(#[future] katana: Katana, _setup: ()) {
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
    let sender_transactions = eth_client.mempool().get_transactions_by_sender_and_nonce(address, 0);
    // Check if the returned transaction hash matches
    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());

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
async fn test_mempool_add_external_transaction(#[future] katana: Katana, _setup: ()) {
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
async fn test_mempool_add_transactions(#[future] katana: Katana, _setup: ()) {
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
async fn test_mempool_add_external_transactions(#[future] katana: Katana, _setup: ()) {
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
    // Check if the returned transaction hash matches
    assert_eq!(*external_transactions[0].hash(), signed_transactions[0].hash());
    assert_eq!(*external_transactions[1].hash(), signed_transactions[1].hash());

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
async fn test_mempool_add_transaction_and_subscribe(#[future] katana: Katana, _setup: ()) {
    let eth_client = katana.eth_client();

    // Create a sample transaction
    let (transaction, _) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Add transaction and subscribe to events
    let result =
        eth_client.mempool().add_transaction_and_subscribe(TransactionOrigin::Local, transaction.clone()).await;
    // Ensure the transaction was added successfully
    assert!(result.is_ok());

    // Get updated mempool size
    let mempool_size = eth_client.mempool().pool_size();
    // Check pending transactions
    assert_eq!(mempool_size.pending, 1);
    // Check queued transactions
    assert_eq!(mempool_size.queued, 0);
    // Check total transactions
    assert_eq!(mempool_size.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_transaction_event_listener(#[future] katana: Katana, _setup: ()) {
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
async fn test_mempool_get_private_transactions(#[future] katana: Katana, _setup: ()) {
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
async fn create_sample_transactions(
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
        let encoded_length = transaction_signed_ec_recovered.clone().length_without_header();

        // Create a pooled transaction
        let eth_pooled_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);

        // Add the transaction to the vector
        transactions.push((eth_pooled_transaction, transaction_signed));
    }
    Ok(transactions)
}
