#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use kakarot_rpc::{
    eth_provider::{
        constant::{
            Constant, ADDRESS_HEX_STRING_LEN, BLOCK_NUMBER_HEX_STRING_LEN, CALL_REQUEST_GAS_LIMIT, HASH_HEX_STRING_LEN,
            LOGS_TOPICS_HEX_STRING_LEN, MAX_LOGS, MAX_PRIORITY_FEE_PER_GAS, STARKNET_MODULUS, U64_HEX_STRING_LEN,
        },
        database::types::transaction::StoredPendingTransaction,
        provider::EthereumProvider,
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
    let expected_constant = Constant {
        max_logs: *MAX_LOGS,
        max_priority_fee_per_gas: *MAX_PRIORITY_FEE_PER_GAS,
        call_request_gas_limit: CALL_REQUEST_GAS_LIMIT,
        hash_hex_string_len: HASH_HEX_STRING_LEN,
        logs_topics_hex_string_len: LOGS_TOPICS_HEX_STRING_LEN,
        u64_hex_string_len: U64_HEX_STRING_LEN,
        block_number_hex_string_len: BLOCK_NUMBER_HEX_STRING_LEN,
        address_hex_string_len: ADDRESS_HEX_STRING_LEN,
        starknet_modulus: STARKNET_MODULUS,
    };
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("kakarot_getConfig").build())
        .send()
        .await
        .expect("kakarot_getConfig error");
    let result_constant: Constant = serde_json::from_str(&res.text().await.expect("Failed to get response body"))
        .and_then(|raw: Value| serde_json::from_value(raw["result"].clone()))
        .expect("Failed to deserialize response body or convert result to Constant");

    assert_eq!(result_constant, expected_constant);

    drop(server_handle);
}
