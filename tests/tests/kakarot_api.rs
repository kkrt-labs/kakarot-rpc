#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use alloy_primitives::B256;
use kakarot_rpc::{
    providers::eth_provider::constant::Constant,
    test_utils::{
        fixtures::{katana, setup},
        katana::Katana,
        rpc::{start_kakarot_rpc_server, RawRpcParamsBuilder},
    },
};
use mockito::Server;
use reth_primitives::{sign_message, Address, Bytes, Transaction, TransactionSigned, TxEip1559, TxKind, B256, U256};
use rstest::*;
use serde_json::Value;
use std::{env, str::FromStr};

#[cfg(feature = "rpc_forwarding")]
#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_send_raw_transaction_rpc_forwarding(#[future] katana: Katana, _setup: ()) {
    let mut server = Server::new();
    let mock_server = server
        .mock("POST", "/")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"jsonrpc":"2.0","id":1,"result":"0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef"}"#,
        )
        .create();

    let (_, _) = start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Set the MAIN_RPC_URL environment variable
    env::set_var("MAIN_RPC_URL", server.url());
    drop(server);

    let eth_client = katana.eth_client();

    // Create a sample raw transaction
    let raw_tx = Bytes::from(vec![1, 2, 3, 4]);

    // Call the function
    let result = eth_client.send_raw_transaction(raw_tx).await;

    // Assert the result
    assert!(result.is_ok());
    let tx_hash = result.unwrap();
    assert_eq!(tx_hash, B256::from_str("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap());

    // Verify that the mock was called
    mock_server.assert();
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_kakarot_get_config(#[future] katana: Katana, _setup: ()) {
    // Define variables
    let starknet_network = "http://0.0.0.0:1010/";
    let white_listed_eip_155_transaction_hashes = "0xe65425dacfc1423823cb4766aa0192ffde61eaa9bf81af9fe15149a89ef36c28";
    let max_logs = 10000;
    let max_felts_in_calldata = 22500;

    // Set environment variables for the test
    std::env::set_var("STARKNET_NETWORK", starknet_network);
    std::env::set_var("WHITE_LISTED_EIP_155_TRANSACTION_HASHES", white_listed_eip_155_transaction_hashes);
    std::env::set_var("MAX_LOGS", max_logs.to_string());
    std::env::set_var("MAX_FELTS_IN_CALLDATA", max_felts_in_calldata.to_string());
    std::env::set_var("KAKAROT_ADDRESS", "0x03d937c035c878245caf64531a5756109c53068da139362728feb561405371cb");

    // Hardcoded expected values
    let expected_constant = Constant {
        max_logs: Some(max_logs),
        starknet_network: (starknet_network).to_string(),
        max_felts_in_calldata,
        white_listed_eip_155_transaction_hashes: vec![B256::from_str(white_listed_eip_155_transaction_hashes).unwrap()],
        kakarot_address: Felt::from_hex("0x03d937c035c878245caf64531a5756109c53068da139362728feb561405371cb").unwrap(),
    };

    // Start the Kakarot RPC server
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the RPC request to get the configuration
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("kakarot_getConfig").build())
        .send()
        .await
        .expect("kakarot_getConfig error");

    // Deserialize the response
    let result_constant: Constant = serde_json::from_str(&res.text().await.expect("Failed to get response body"))
        .and_then(|raw: Value| serde_json::from_value(raw["result"].clone()))
        .expect("Failed to deserialize response body or convert result to Constant");

    // Assert that the returned configuration matches the expected value
    assert_eq!(result_constant, expected_constant);

    drop(server_handle);
}
