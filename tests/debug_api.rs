#![cfg(feature = "testing")]
use kakarot_rpc::test_utils::fixtures::{katana, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::mongo::{BLOCK_HASH, BLOCK_NUMBER, EIP1599_TX_HASH, EIP2930_TX_HASH, LEGACY_TX_HASH};
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use reth_rpc_types_compat::transaction::from_recovered_with_block_context;

use reth_primitives::{Bytes, TransactionSigned, TransactionSignedEcRecovered, U256};
use rstest::*;
use serde_json::{json, Value};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_raw_transaction(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // EIP1559
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawTransaction",
                    "params":[format!("0x{:064x}", *EIP1599_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rlp_bytes: Option<Bytes> = serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");
    assert!(rlp_bytes.is_some());
    // We can decode the RLP bytes to get the transaction and compare it with the original transaction
    let transaction = TransactionSigned::decode_enveloped(&mut rlp_bytes.unwrap().as_ref()).unwrap();
    let signer = transaction.recover_signer().unwrap();
    let transaction = from_recovered_with_block_context(
        TransactionSignedEcRecovered::from_signed_transaction(transaction, signer),
        *BLOCK_HASH,
        BLOCK_NUMBER,
        None,
        U256::ZERO,
    );
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"eth_getTransactionByHash",
                    "params":[format!("0x{:064x}", *EIP1599_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let response: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rpc_transaction: reth_rpc_types::Transaction =
        serde_json::from_value(response["result"].clone()).expect("Failed to deserialize result");
    assert_eq!(transaction, rpc_transaction);

    // EIP2930
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawTransaction",
                    "params":[format!("0x{:064x}", *EIP2930_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rlp_bytes: Option<Bytes> = serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");
    assert!(rlp_bytes.is_some());
    // We can decode the RLP bytes to get the transaction and compare it with the original transaction
    let transaction = TransactionSigned::decode_enveloped(&mut rlp_bytes.unwrap().as_ref()).unwrap();
    let signer = transaction.recover_signer().unwrap();
    let transaction = from_recovered_with_block_context(
        TransactionSignedEcRecovered::from_signed_transaction(transaction, signer),
        *BLOCK_HASH,
        BLOCK_NUMBER,
        None,
        U256::ZERO,
    );
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"eth_getTransactionByHash",
                    "params":[format!("0x{:064x}", *EIP2930_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let response: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rpc_transaction: reth_rpc_types::Transaction =
        serde_json::from_value(response["result"].clone()).expect("Failed to deserialize result");
    assert_eq!(transaction, rpc_transaction);

    // Legacy
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawTransaction",
                    "params":[format!("0x{:064x}", *LEGACY_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rlp_bytes: Option<Bytes> = serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");
    assert!(rlp_bytes.is_some());
    // We can decode the RLP bytes to get the transaction and compare it with the original transaction
    let transaction = TransactionSigned::decode_enveloped(&mut rlp_bytes.unwrap().as_ref()).unwrap();
    let signer = transaction.recover_signer().unwrap();
    let transaction = from_recovered_with_block_context(
        TransactionSignedEcRecovered::from_signed_transaction(transaction, signer),
        *BLOCK_HASH,
        BLOCK_NUMBER,
        None,
        U256::ZERO,
    );
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"eth_getTransactionByHash",
                    "params":[format!("0x{:064x}", *LEGACY_TX_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let response: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let rpc_transaction: reth_rpc_types::Transaction =
        serde_json::from_value(response["result"].clone()).expect("Failed to deserialize result");
    assert_eq!(transaction, rpc_transaction);

    drop(server_handle);
}
