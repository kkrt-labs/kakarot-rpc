#![cfg(feature = "testing")]
use alloy_rlp::{Decodable, Encodable};
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::models::block::rpc_to_primitive_block;
use kakarot_rpc::models::transaction::rpc_transaction_to_primitive;
use kakarot_rpc::test_utils::fixtures::{katana, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::mongo::{BLOCK_HASH, BLOCK_NUMBER, EIP1599_TX_HASH, EIP2930_TX_HASH, LEGACY_TX_HASH};
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use reth_primitives::{
    BlockNumberOrTag, Bytes, Log, Receipt, ReceiptWithBloom, TransactionSigned, TransactionSignedEcRecovered, U256,
};
use reth_rpc_types_compat::transaction::from_recovered_with_block_context;
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

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
/// Test for fetching raw transactions by block hash and block number.
async fn test_raw_transactions(#[future] katana: Katana, _setup: ()) {
    // Start the Kakarot RPC server.
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Fetch raw transactions by block hash.
    let reqwest_client = reqwest::Client::new();
    let res_by_block_hash = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawTransactions",
                    "params":[format!("0x{:064x}", *BLOCK_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response_by_block_hash = res_by_block_hash.text().await.expect("Failed to get response body");
    let raw_by_block_hash: Value =
        serde_json::from_str(&response_by_block_hash).expect("Failed to deserialize response body");

    let rlp_bytes_by_block_hash: Vec<Bytes> =
        serde_json::from_value(raw_by_block_hash["result"].clone()).expect("Failed to deserialize result");

    // Fetch raw transactions by block number.
    let res_by_block_number = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawTransactions",
                    "params":[format!("0x{:064x}", BLOCK_NUMBER)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response_by_block_number = res_by_block_number.text().await.expect("Failed to get response body");
    let raw_by_block_number: Value =
        serde_json::from_str(&response_by_block_number).expect("Failed to deserialize response body");

    let rlp_bytes_by_block_number: Vec<Bytes> =
        serde_json::from_value(raw_by_block_number["result"].clone()).expect("Failed to deserialize result");

    // Assert equality of transactions fetched by block hash and block number.
    assert_eq!(rlp_bytes_by_block_number, rlp_bytes_by_block_hash);

    let eth_provider = katana.eth_provider();

    for (i, actual_tx) in eth_provider
        .block_transactions(Some(reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(BLOCK_NUMBER))))
        .await
        .unwrap()
        .unwrap()
        .iter()
        .enumerate()
    {
        // Fetch the transaction for the current transaction hash.
        let tx = eth_provider.transaction_by_hash(actual_tx.hash).await.unwrap().unwrap();
        let signature = tx.signature.unwrap();

        // Convert the transaction to a primitives transactions and encode it.
        let rlp_bytes = TransactionSigned::from_transaction_and_signature(
            rpc_transaction_to_primitive(tx).unwrap(),
            reth_primitives::Signature {
                r: signature.r,
                s: signature.s,
                odd_y_parity: signature.y_parity.unwrap_or(reth_rpc_types::Parity(false)).0,
            },
        )
        .envelope_encoded();

        // Assert the equality of the constructed receipt with the corresponding receipt from both block hash and block number.
        assert_eq!(rlp_bytes_by_block_number[i], rlp_bytes);
        assert_eq!(rlp_bytes_by_block_hash[i], rlp_bytes);
    }

    // Stop the Kakarot RPC server.
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
/// Test for fetching raw receipts by block hash and block number.
async fn test_raw_receipts(#[future] katana: Katana, _setup: ()) {
    // Start the Kakarot RPC server.
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Fetch raw receipts by block hash.
    let reqwest_client = reqwest::Client::new();
    let res_by_block_hash = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawReceipts",
                    "params":[format!("0x{:064x}", *BLOCK_HASH)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response_by_block_hash = res_by_block_hash.text().await.expect("Failed to get response body");
    let raw_by_block_hash: Value =
        serde_json::from_str(&response_by_block_hash).expect("Failed to deserialize response body");

    let rlp_bytes_by_block_hash: Vec<Bytes> =
        serde_json::from_value(raw_by_block_hash["result"].clone()).expect("Failed to deserialize result");

    // Fetch raw receipts by block number.
    let res_by_block_number = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawReceipts",
                    "params":[format!("0x{:064x}", BLOCK_NUMBER)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response_by_block_number = res_by_block_number.text().await.expect("Failed to get response body");
    let raw_by_block_number: Value =
        serde_json::from_str(&response_by_block_number).expect("Failed to deserialize response body");

    let rlp_bytes_by_block_number: Vec<Bytes> =
        serde_json::from_value(raw_by_block_number["result"].clone()).expect("Failed to deserialize result");

    // Assert equality of receipts fetched by block hash and block number.
    assert_eq!(rlp_bytes_by_block_number, rlp_bytes_by_block_hash);

    // Get eth provider
    let eth_provider = katana.eth_provider();

    for (i, receipt) in eth_provider
        .block_receipts(Some(reth_rpc_types::BlockId::Number(BlockNumberOrTag::Number(BLOCK_NUMBER))))
        .await
        .unwrap()
        .unwrap()
        .iter()
        .enumerate()
    {
        // Fetch the transaction receipt for the current receipt hash.
        let tx_receipt = eth_provider.transaction_receipt(receipt.transaction_hash.unwrap()).await.unwrap().unwrap();

        // Construct a Receipt instance from the transaction receipt data.
        let r = ReceiptWithBloom {
            receipt: Receipt {
                tx_type: tx_receipt.transaction_type.to::<u8>().try_into().unwrap(),
                success: tx_receipt.status_code.unwrap_or_default().to::<u64>() == 1,
                cumulative_gas_used: tx_receipt.cumulative_gas_used.to::<u64>(),
                logs: tx_receipt
                    .logs
                    .into_iter()
                    .map(|log| Log { address: log.address, topics: log.topics, data: log.data })
                    .collect(),
            },
            bloom: tx_receipt.logs_bloom,
        }
        .envelope_encoded();

        // Assert the equality of the constructed receipt with the corresponding receipt from both block hash and block number.
        assert_eq!(rlp_bytes_by_block_number[i], r);
        assert_eq!(rlp_bytes_by_block_hash[i], r);
    }

    // Stop the Kakarot RPC server.
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_raw_block(#[future] katana: Katana, _setup: ()) {
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_getRawBlock",
                    "params":[format!("0x{:064x}", BLOCK_NUMBER)],
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

    // Query the block with eth_getBlockByNumber
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"eth_getBlockByNumber",
                    "params":[format!("0x{:x}", BLOCK_NUMBER), true],
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
    let rpc_block: reth_rpc_types::Block =
        serde_json::from_value(response["result"].clone()).expect("Failed to deserialize result");
    let primitive_block = rpc_to_primitive_block(rpc_block).unwrap();

    // Encode primitive block and compare with the result of debug_getRawBlock
    let mut buf = Vec::new();
    primitive_block.encode(&mut buf);
    assert_eq!(rlp_bytes.clone().unwrap(), Bytes::from(buf));

    // Decode encoded block and compare with the block from eth_getBlockByNumber
    let decoded_block = reth_primitives::Block::decode(&mut rlp_bytes.unwrap().as_ref()).unwrap();
    assert_eq!(decoded_block, primitive_block);

    // Stop the Kakarot RPC server.
    drop(server_handle);
}
