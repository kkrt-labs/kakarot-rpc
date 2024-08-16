#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use jsonrpsee::server::ServerHandle;
use kakarot_rpc::{
    providers::eth_provider::database::types::transaction::StoredPendingTransaction,
    test_utils::{
        fixtures::{katana, setup},
        katana::Katana,
        mongo::RANDOM_BYTES_SIZE,
        rpc::{start_kakarot_rpc_server, RawRpcParamsBuilder},
    },
};
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus};
use rstest::*;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::net::SocketAddr;

async fn initial_setup(katana: Katana) -> (SocketAddr, ServerHandle, Katana) {
    // Start the Kakarot RPC server and retrieve the server address and handle
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Generate a vector of random bytes
    let bytes: Vec<u8> = (0..RANDOM_BYTES_SIZE).map(|_| rand::random()).collect();
    let mut unstructured = arbitrary::Unstructured::new(&bytes);

    // Generate 10 pending transactions and add them to the database
    let mut pending_transactions = Vec::new();
    for _ in 0..10 {
        pending_transactions
            .push(StoredPendingTransaction::arbitrary_with_optional_fields(&mut unstructured).unwrap().tx);
    }
    katana.add_pending_transactions_to_database(pending_transactions).await;
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
async fn test_txpool_content(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle, katana) = initial_setup(katana).await;

    let tx_pool_content: TxpoolContent = request("txpool_content", server_addr.port(), Vec::<String>::new()).await;

    // Assert that we recovered the 10 pending transactions
    assert_eq!(tx_pool_content.pending.len(), 10);

    // Assert that no queued transactions are registered
    assert!(tx_pool_content.queued.is_empty());

    // Retrieve the first pending transaction from the database
    let first_pending_tx = katana
        .eth_provider()
        .database()
        .get_first::<StoredPendingTransaction>()
        .await
        .expect("Failed to get the first pending transaction")
        .unwrap();

    // Assert that the pool content contains the sender of the first pending transaction
    assert!(tx_pool_content.pending.contains_key(&first_pending_tx.from));

    // Check that the first transaction in the pool matches the first pending transaction
    assert_eq!(
        *tx_pool_content.pending.get(&first_pending_tx.from).unwrap().get(&first_pending_tx.nonce.to_string()).unwrap(),
        first_pending_tx.into()
    );

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_content_from(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle, katana) = initial_setup(katana).await;

    // Retrieve the first pending transaction from the database
    let first_pending_tx = katana
        .eth_provider()
        .database()
        .get_first::<StoredPendingTransaction>()
        .await
        .expect("Failed to get the first pending transaction")
        .unwrap();

    let tx_pool_content: TxpoolContentFrom =
        request("txpool_contentFrom", server_addr.port(), vec![first_pending_tx.tx.from.to_string()]).await;

    // Assert that we recovered a single pending transaction
    assert_eq!(tx_pool_content.pending.len(), 1);

    // Assert that no queued transactions are registered
    assert!(tx_pool_content.queued.is_empty());

    // Assert the validity of the recovered pending transaction
    assert_eq!(*tx_pool_content.pending.get(&first_pending_tx.nonce.to_string()).unwrap(), first_pending_tx.tx);

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_status(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle, katana) = initial_setup(katana).await;

    let tx_pool_status: TxpoolStatus = request("txpool_status", server_addr.port(), Vec::<String>::new()).await;

    // Assert that we recovered the 10 pending transactions
    assert_eq!(tx_pool_status.pending, 10);

    // Assert that no queued transactions are registered
    assert_eq!(tx_pool_status.queued, 0);

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
    drop(katana);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_txpool_inspect(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle, katana) = initial_setup(katana).await;

    let tx_pool_inspect: TxpoolInspect = request("txpool_inspect", server_addr.port(), Vec::<String>::new()).await;

    // Assert that we recovered the 10 pending transactions
    assert_eq!(tx_pool_inspect.pending.len(), 10);

    // Assert that no queued transactions are registered
    assert!(tx_pool_inspect.queued.is_empty());

    // Retrieve the first pending transaction from the database
    let first_pending_tx = katana
        .eth_provider()
        .database()
        .get_first::<StoredPendingTransaction>()
        .await
        .expect("Failed to get the first pending transaction")
        .unwrap();

    // Assert that the pool content contains the sender of the first pending transaction
    assert!(tx_pool_inspect.pending.contains_key(&first_pending_tx.from));

    // Check that the first transaction in the pool matches the first pending transaction
    assert_eq!(
        *tx_pool_inspect.pending.get(&first_pending_tx.from).unwrap().get(&first_pending_tx.nonce.to_string()).unwrap(),
        TxpoolInspectSummary {
            to: first_pending_tx.to,
            value: first_pending_tx.value,
            gas: first_pending_tx.gas,
            gas_price: first_pending_tx.gas_price.unwrap()
        }
    );

    // Drop the server handle to shut down the server after the test
    drop(server_handle);
}
