#![cfg(feature = "testing")]
use kakarot_rpc::test_utils::fixtures::{katana, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use rstest::*;
use serde_json::{json, Value};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_block_number(#[future] katana: Katana, _setup: ()) {
    // Start Kakarot RPC server and get its address and handle.
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Get the most recent block number.
    let expected_block_number = katana.most_recent_transaction().unwrap().block_number;

    // Create a reqwest client.
    let reqwest_client = reqwest::Client::new();

    // Fetch the most recent block number from the Kakarot RPC server.
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc": "2.0",
                    "method": "eth_blockNumber",
                    "params": [],
                    "id": 1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Eth RPC");
    let response = res.text().await.expect("Failed to get response body");

    // Deserialize response body and extract block number
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let block_number: Option<u64> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    // Assert that the fetched block number matches the expected block number.
    assert_eq!(block_number, expected_block_number);

    // Stop the Kakarot RPC server.
    drop(server_handle);
}
