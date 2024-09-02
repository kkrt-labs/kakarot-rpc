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
    sign_message, Address, Bytes, Transaction, TransactionSigned, TransactionSignedEcRecovered, TxEip1559, TxKind, U256,
};
use reth_transaction_pool::{EthPooledTransaction, TransactionOrigin, TransactionPool};
use rstest::*;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transaction(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, _) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    assert_eq!(eth_provider.mempool().unwrap().pool_size().total, 0);

    let result = eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await;

    assert!(result.is_ok());
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_external_transaction(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, _) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    let result = eth_provider.mempool().unwrap().add_external_transaction(transaction).await;
    assert!(result.is_ok());
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");
    let transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, transactions).await;
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 2);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 2);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_external_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");
    let transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_external_transactions(transactions).await;
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 2);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 2);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_add_transaction_and_subscribe(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, _) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    let result = eth_provider
        .mempool()
        .unwrap()
        .add_transaction_and_subscribe(TransactionOrigin::Local, transaction.clone())
        .await;

    assert!(result.is_ok());
    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_transaction_event_listener(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();

    let listener = eth_provider.mempool().unwrap().transaction_event_listener(transaction_signed.hash());
    assert!(listener.is_some());
    assert_eq!(listener.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_pooled_transaction_hashes(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions).await;

    let hashes = eth_provider.mempool().unwrap().pooled_transaction_hashes();
    assert!(hashes.contains(&signed_transactions[0].hash()));
    assert!(hashes.contains(&signed_transactions[1].hash()));
    assert!(!hashes.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_pooled_transaction_hashes_max(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions).await;

    let hashes_max = 1;
    let hashes = eth_provider.mempool().unwrap().pooled_transaction_hashes_max(hashes_max);
    assert!(hashes.contains(&signed_transactions[0].hash()) || hashes.contains(&signed_transactions[1].hash()));
    assert_eq!(hashes.len(), hashes_max);
    assert!(!hashes.is_empty());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_pooled_transaction(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transactions = create_sample_transactions(&katana, 2).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions).await;

    let hashes = eth_provider.mempool().unwrap().get_pooled_transaction_element(signed_transactions[0].hash());
    assert_eq!(hashes.unwrap().hash(), &signed_transactions[0].hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_pending_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transaction_number = 2;
    let transactions =
        create_sample_transactions(&katana, transaction_number).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    let _ = eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions).await;

    let hashes = eth_provider.mempool().unwrap().pending_transactions();
    assert_eq!(hashes[0].hash(), &signed_transactions[0].hash());
    assert_eq!(hashes[1].hash(), &signed_transactions[1].hash());
    assert_eq!(hashes.len(), transaction_number);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_transactions_by_sender(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender(address);

    assert_eq!(sender_transactions.len(), 1);
    assert_eq!(*sender_transactions[0].hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_transactions_by_sender_and_nonce(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender_and_nonce(address, 0);

    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn get_transactions_by_origin(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender_and_nonce(address, 0);

    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_local_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender_and_nonce(address, 0);

    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_private_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Private, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender_and_nonce(address, 0);

    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_get_external_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::External, transaction.clone()).await.unwrap();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let sender_transactions = eth_provider.mempool().unwrap().get_transactions_by_sender_and_nonce(address, 0);

    assert_eq!(*sender_transactions.unwrap().hash(), transaction_signed.hash());
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_remove_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();

    assert_eq!(eth_provider.mempool().unwrap().pool_size().total, 1);

    let removed_transactions = eth_provider.mempool().unwrap().remove_transactions(vec![transaction_signed.hash()]);

    assert_eq!(removed_transactions.len(), 1);

    let mempool_size = eth_provider.mempool().unwrap().pool_size();
    assert_eq!(mempool_size.pending, 0);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_all_transactions(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let (transaction, transaction_signed) = create_sample_transactions(&katana, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");
    eth_provider.mempool().unwrap().add_transaction(TransactionOrigin::Local, transaction.clone()).await.unwrap();

    let all_transactions = eth_provider.mempool().unwrap().all_transactions();

    assert_eq!(*all_transactions.pending[0].hash(), transaction_signed.hash());
    assert_eq!(all_transactions.pending.len(), 1);
    assert_eq!(all_transactions.queued.len(), 0);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_retain_unknown(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transaction_number = 2;
    let transactions =
        create_sample_transactions(&katana, transaction_number).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let signed_transactions =
        transactions.iter().map(|(_, signed_transactions)| signed_transactions.clone()).collect::<Vec<_>>();

    let _ =
        eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions.clone()).await;

    let contains = eth_provider.mempool().unwrap().contains(&signed_transactions[0].hash());
    assert!(contains);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_mempool_unique_senders(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();
    let transaction_number = 2;
    let transactions =
        create_sample_transactions(&katana, transaction_number).await.expect("Failed to create sample transaction");
    let pooled_transactions =
        transactions.iter().map(|(eth_pooled_transaction, _)| eth_pooled_transaction.clone()).collect::<Vec<_>>();
    let address = katana.eoa().evm_address().expect("Failed to get eoa address");

    let _ =
        eth_provider.mempool().unwrap().add_transactions(TransactionOrigin::Local, pooled_transactions.clone()).await;

    let unique_senders = eth_provider.mempool().unwrap().unique_senders();

    assert!(unique_senders.contains(&address));
}

// Helper function to create a sample transaction
async fn create_sample_transactions(
    katana: &Katana,
    num_transactions: usize,
) -> Result<Vec<(EthPooledTransaction, TransactionSigned)>, SignatureError> {
    let mut transactions = Vec::new();
    for counter in 0..num_transactions {
        let eth_provider = katana.eth_provider();
        let chain_id = eth_provider.chain_id().await.unwrap_or_default().unwrap_or_default().to();

        let transaction = Transaction::Eip1559(TxEip1559 {
            chain_id,
            nonce: counter as u64,
            gas_limit: 21000,
            to: TxKind::Call(Address::random()),
            value: U256::from(1000),
            input: Bytes::default(),
            max_fee_per_gas: 875_000_000,
            max_priority_fee_per_gas: 0,
            access_list: Default::default(),
        });

        let signature = sign_message(katana.eoa().private_key(), transaction.signature_hash()).unwrap();
        let transaction_signed = TransactionSigned::from_transaction_and_signature(transaction, signature);
        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;
        let transaction_signed_ec_recovered =
            TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer);

        let encoded_length = transaction_signed_ec_recovered.clone().length_without_header();
        let eth_pooled_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);
        transactions.push((eth_pooled_transaction, transaction_signed));
    }
    Ok(transactions)
}
