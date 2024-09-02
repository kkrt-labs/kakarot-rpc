#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use kakarot_rpc::{
    providers::eth_provider::{
        constant::Constant, database::types::transaction::StoredPendingTransaction, ChainProvider, TransactionProvider,
    },
    test_utils::{
        eoa::Eoa,
        fixtures::{katana, setup},
        katana::Katana,
        rpc::{start_kakarot_rpc_server, RawRpcParamsBuilder},
    },
};
use reth_primitives::{sign_message, Address, Bytes, Transaction, TransactionSigned, TxEip1559, TxKind, B256, U256};
use rstest::*;
use serde_json::Value;
use std::str::FromStr;
#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_kakarot_get_starknet_transaction_hash(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

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

    // Send the transaction
    let tx_return = eth_provider
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

    let hash = tx.hash;
    let retries: u8 = 0;

    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("kakarot_getStarknetTransactionHash").add_param(hash).add_param(retries).build())
        .send()
        .await
        .expect("kakarot_getStarknetTransactionHash error");
    let result_starknet_transaction_hash: B256 =
        serde_json::from_str(&res.text().await.expect("Failed to get response body"))
            .and_then(|raw: Value| serde_json::from_value(raw["result"].clone()))
            .expect("Failed to deserialize result");

    assert_eq!(result_starknet_transaction_hash, tx_return);

    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_kakarot_get_starknet_transaction_hash_with_none_tx_hash(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

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

    let hash = transaction_signed.hash();
    let retries: u8 = 0;

    let reqwest_client = reqwest::Client::new();
    let res: reqwest::Response = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("kakarot_getStarknetTransactionHash").add_param(hash).add_param(retries).build())
        .send()
        .await
        .expect("kakarot_getStarknetTransactionHash error");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let result_starknet_transaction_hash = raw["result"].as_str();

    assert_eq!(result_starknet_transaction_hash, None);

    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_kakarot_get_config(#[future] katana: Katana, _setup: ()) {
    // Define variables
    let starknet_network = "http://0.0.0.0:1010/";
    let white_listed_eip_155_transaction_hashes = "0xe65425dacfc1423823cb4766aa0192ffde61eaa9bf81af9fe15149a89ef36c28";
    let max_logs = 10000;
    let poll_tx_interval = 10;
    let max_felts_in_calldata = 22500;

    // Set environment variables for the test
    std::env::set_var("STARKNET_NETWORK", starknet_network);
    std::env::set_var("WHITE_LISTED_EIP_155_TRANSACTION_HASHES", white_listed_eip_155_transaction_hashes);
    std::env::set_var("MAX_LOGS", max_logs.to_string());
    std::env::set_var("PROCESS_PENDING_TX_INTERVAL", poll_tx_interval.to_string());
    std::env::set_var("MAX_FELTS_IN_CALLDATA", max_felts_in_calldata.to_string());

    // Hardcoded expected values
    let expected_constant = Constant {
        max_logs: Some(max_logs),
        starknet_network: (starknet_network).to_string(),
        poll_tx_interval,
        max_felts_in_calldata,
        white_listed_eip_155_transaction_hashes: vec![B256::from_str(white_listed_eip_155_transaction_hashes).unwrap()],
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
