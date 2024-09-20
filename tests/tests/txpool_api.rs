#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use crate::tests::mempool::create_sample_transactions;
use jsonrpsee::server::ServerHandle;
use kakarot_rpc::test_utils::{
    fixtures::{katana_empty, setup},
    katana::Katana,
    rpc::{start_kakarot_rpc_server, RawRpcParamsBuilder},
};
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus};
use reth_transaction_pool::{TransactionOrigin, TransactionPool};
use rstest::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

async fn initial_setup(katana: Katana) -> (SocketAddr, ServerHandle, Katana) {
    // Start the Kakarot RPC server and retrieve the server address and handle
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    (server_addr, server_handle, katana)
}

async fn request<D: DeserializeOwned, S: Serialize>(method: &str, port: u16, params: Vec<S>) -> D {
    // Create a reqwest client
    let reqwest_client = reqwest::Client::new();

    // Build the JSON-RPC request body
    let mut body_builder = RawRpcParamsBuilder::new(method);
    for p in params {
        body_builder = body_builder.add_param(p);
    }
    let body = body_builder.build();

    // Send a POST request to the Kakarot RPC server
    let res = reqwest_client
        .post(format!("http://localhost:{port}"))
        .header("Content-Type", "application/json")
        .body(body)
        .send()
        .await
        .expect("Failed to call TxPool RPC");

    // Extract the response body as text
    let response = res.text().await.expect("Failed to get response body");
    // Deserialize the response body into JSON
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    // Deserialize the 'result' field of the JSON into a T struct
    serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result")
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_content(#[future] katana_empty: Katana, _setup: ()) {
    let (server_addr, server_handle, katana_empty) = initial_setup(katana_empty).await;

    // Create a sample transactions
    let (transaction, transaction_signed) = create_sample_transactions(&katana_empty, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Insert the transaction into the mempool
    let _tx_hash = katana_empty
        .eth_client
        .mempool()
        .add_transaction(TransactionOrigin::Local, transaction)
        .await
        .expect("Failed to insert transaction into the mempool");

    // Fetch the transaction pool content
    let tx_pool_content: TxpoolContent = request("txpool_content", server_addr.port(), Vec::<String>::new()).await;

    // Get updated mempool size
    let mempool_size = katana_empty.eth_client.mempool().pool_size();
    // Check pending, queued and total transactions
    assert_eq!(mempool_size.pending, 1);
    assert_eq!(mempool_size.queued, 0);
    assert_eq!(mempool_size.total, 1);

    // Recover the signer from the transaction
    let transaction_signer = transaction_signed.recover_signer().unwrap();

    // Assert that the pool content contains the sender of the first pending transaction
    assert!(tx_pool_content.pending.contains_key(&transaction_signer));

    // Check that the transaction in the pool matches the pending transaction that was inserted
    assert_eq!(
        *tx_pool_content
            .pending
            .get(&transaction_signer)
            .unwrap()
            .get(&transaction_signed.transaction.nonce().to_string())
            .unwrap()
            .hash,
        transaction_signed.hash()
    );

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_content_from(#[future] katana_empty: Katana, _setup: ()) {
    let (server_addr, server_handle, katana_empty) = initial_setup(katana_empty).await;

    // Create a sample transactions
    let (transaction, transaction_signed) = create_sample_transactions(&katana_empty, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Insert the transaction into the mempool
    let _tx_hash = katana_empty
        .eth_client
        .mempool()
        .add_transaction(TransactionOrigin::Local, transaction)
        .await
        .expect("Failed to insert transaction into the mempool");

    // Recover the signer from the transaction
    let transaction_signer = transaction_signed.recover_signer().unwrap();

    // Fetch the transaction pool content from the sender
    let tx_pool_content: TxpoolContentFrom =
        request("txpool_contentFrom", server_addr.port(), vec![transaction_signer.to_string()]).await;

    // Assert that we recovered a single pending transaction
    assert_eq!(tx_pool_content.pending.len(), 1);

    // Assert that no queued transactions are registered
    assert!(tx_pool_content.queued.is_empty());

    // Assert the validity of the recovered pending transaction
    assert_eq!(
        *tx_pool_content.pending.get(&transaction_signed.transaction.nonce().to_string()).unwrap().hash,
        transaction_signed.hash()
    );

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_status(#[future] katana_empty: Katana, _setup: ()) {
    let (server_addr, server_handle, katana_empty) = initial_setup(katana_empty).await;

    // Create a sample transactions
    let (transaction, _) = create_sample_transactions(&katana_empty, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Insert the transaction into the mempool
    let _tx_hash = katana_empty
        .eth_client
        .mempool()
        .add_transaction(TransactionOrigin::Local, transaction)
        .await
        .expect("Failed to insert transaction into the mempool");

    // Fetch the transaction pool status
    let tx_pool_status: TxpoolStatus = request("txpool_status", server_addr.port(), Vec::<String>::new()).await;

    // Assert that we recovered the pending transaction
    assert_eq!(tx_pool_status.pending, 1);

    // Assert that no queued transactions are registered
    assert_eq!(tx_pool_status.queued, 0);

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
    drop(katana_empty);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_inspect(#[future] katana_empty: Katana, _setup: ()) {
    let (server_addr, server_handle, katana_empty) = initial_setup(katana_empty).await;

    // Create a sample transactions
    let (transaction, transaction_signed) = create_sample_transactions(&katana_empty, 1)
        .await
        .expect("Failed to create sample transaction")
        .pop()
        .expect("Expected at least one transaction");

    // Insert the transaction into the mempool
    let _tx_hash = katana_empty
        .eth_client
        .mempool()
        .add_transaction(TransactionOrigin::Local, transaction)
        .await
        .expect("Failed to insert transaction into the mempool");

    // Inspect the transaction pool
    let tx_pool_inspect: TxpoolInspect = request("txpool_inspect", server_addr.port(), Vec::<String>::new()).await;

    // Assert that we recovered the pending transaction
    assert_eq!(tx_pool_inspect.pending.len(), 1);

    // Assert that no queued transactions are registered
    assert!(tx_pool_inspect.queued.is_empty());

    // Recover the signer from the transaction
    let transaction_signer = transaction_signed.recover_signer().unwrap();

    // Assert that the pool content contains the sender of the first pending transaction
    assert!(tx_pool_inspect.pending.contains_key(&transaction_signer));

    // Check that the first transaction in the pool matches the first pending transaction
    assert_eq!(
        *tx_pool_inspect
            .pending
            .get(&transaction_signer)
            .unwrap()
            .get(&transaction_signed.transaction.nonce().to_string())
            .unwrap(),
        TxpoolInspectSummary {
            to: transaction_signed.to(),
            value: transaction_signed.value(),
            gas: transaction_signed.gas_limit() as u128,
            gas_price: transaction_signed.max_fee_per_gas(),
        }
    );

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}
