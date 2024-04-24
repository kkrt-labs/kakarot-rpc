#![cfg(feature = "testing")]
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::test_utils::eoa::Eoa;
use kakarot_rpc::test_utils::evm_contract::{
    EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo,
};
use kakarot_rpc::test_utils::fixtures::{plain_opcodes, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use reth_primitives::{B256, U256};
use reth_rpc_types::trace::geth::TraceResult;
use reth_rpc_types::trace::parity::LocalizedTransactionTrace;
use rstest::*;
use serde_json::{json, Value};
use starknet::core::types::MaybePendingBlockWithTxHashes;
use starknet::providers::Provider;

const TRACING_BLOCK_NUMBER: u64 = 0x3;
const TRANSACTIONS_COUNT: usize = 1;

/// Helper to create a header.
fn header(block_number: u64, hash: B256, parent_hash: B256, base_fee: u128) -> reth_rpc_types::Header {
    reth_rpc_types::Header {
        number: Some(block_number),
        hash: Some(hash),
        parent_hash,
        nonce: Default::default(),
        logs_bloom: Default::default(),
        transactions_root: Default::default(),
        state_root: Default::default(),
        receipts_root: Default::default(),
        difficulty: Default::default(),
        total_difficulty: Default::default(),
        extra_data: Default::default(),
        gas_limit: u64::MAX as u128,
        gas_used: Default::default(),
        timestamp: Default::default(),
        uncles_hash: Default::default(),
        miner: Default::default(),
        mix_hash: Default::default(),
        base_fee_per_gas: Some(base_fee),
        withdrawals_root: Default::default(),
        excess_blob_gas: Default::default(),
        parent_beacon_block_root: Default::default(),
        blob_gas_used: Default::default(),
    }
}

/// Helper to set up the debug/tracing environment on Katana.
pub async fn tracing(katana: &Katana, plain_opcodes: &KakarotEvmContract) {
    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get eoa address");
    let nonce: u64 = eoa.nonce().await.expect("Failed to get nonce").to();
    let chain_id = eoa.eth_provider().chain_id().await.expect("Failed to get chain id").unwrap_or_default().to();

    // Push 10 RPC transactions into the database.
    let mut txs = Vec::with_capacity(TRANSACTIONS_COUNT);
    let max_fee_per_gas = 10;
    let max_priority_fee_per_gas = 1;
    for i in 0..TRANSACTIONS_COUNT {
        // We want to trace the "createCounterAndInvoke" which does a CREATE followed by a CALL.
        let tx = plain_opcodes
            .prepare_call_transaction(
                "createCounterAndInvoke",
                (),
                &TransactionInfo::FeeMarketInfo(TxFeeMarketInfo {
                    common: TxCommonInfo { nonce: nonce + i as u64, value: 0, chain_id },
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                }),
            )
            .expect("Failed to prepare call transaction");
        // Sign the transaction and convert it to a RPC transaction.
        let tx_signed = eoa.sign_transaction(tx.clone()).expect("Failed to sign transaction");
        let tx = reth_rpc_types::Transaction {
            transaction_type: Some(2),
            nonce: tx.nonce(),
            hash: tx_signed.hash(),
            to: tx.to(),
            from: eoa_address,
            block_number: Some(TRACING_BLOCK_NUMBER),
            chain_id: tx.chain_id(),
            gas: tx.gas_limit() as u128,
            input: tx.input().clone(),
            signature: Some(reth_rpc_types::Signature {
                r: tx_signed.signature().r,
                s: tx_signed.signature().s,
                v: U256::from(tx_signed.signature().v(Some(chain_id))),
                y_parity: Some(reth_rpc_types::Parity(tx_signed.signature().odd_y_parity)),
            }),
            max_fee_per_gas: Some(max_fee_per_gas),
            gas_price: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            value: tx.value(),
            ..Default::default()
        };
        txs.push(tx);
    }

    // Add a block header pointing to a parent hash header in the database for these transactions.
    // This is required since tracing will start on the previous block.
    let maybe_parent_block = katana
        .eth_provider()
        .starknet_provider()
        .get_block_with_tx_hashes(starknet::core::types::BlockId::Number(TRACING_BLOCK_NUMBER - 1))
        .await
        .expect("Failed to get block");
    let parent_block_hash = match maybe_parent_block {
        MaybePendingBlockWithTxHashes::PendingBlock(_) => panic!("Pending block found"),
        MaybePendingBlockWithTxHashes::Block(block) => block.block_hash,
    };
    let parent_block_hash = B256::from_slice(&parent_block_hash.to_bytes_be()[..]);

    let parent_header = header(TRACING_BLOCK_NUMBER - 1, parent_block_hash, B256::random(), max_fee_per_gas);
    let header = header(TRACING_BLOCK_NUMBER, B256::random(), parent_block_hash, max_fee_per_gas);
    katana.add_transactions_with_header_to_database(vec![], parent_header).await;
    katana.add_transactions_with_header_to_database(txs, header).await;
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_block(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    // Setup the Kakarot RPC server.
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the trace_block RPC request.
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"trace_block",
                    "params":[format!("0x{:016x}", TRACING_BLOCK_NUMBER)],
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
    let traces: Option<Vec<LocalizedTransactionTrace>> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    assert!(traces.is_some());
    // We expect 3 traces per transaction: CALL, CREATE, and CALL.
    assert!(traces.unwrap().len() == TRANSACTIONS_COUNT * 3);
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_debug_trace_block_by_number(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    // Setup the Kakarot RPC server.
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the trace_block RPC request.
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"debug_traceBlockByNumber",
                    "params":[format!("0x{:016x}", TRACING_BLOCK_NUMBER), {"tracer": "callTracer", "tracerConfig": {"onlyTopCall": false}, "timeout": "300s"}],
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
    let traces: Option<Vec<TraceResult>> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    assert!(traces.is_some());
    // We expect 3 traces per transaction: CALL, CREATE, and CALL.
    assert!(traces.unwrap().len() == TRANSACTIONS_COUNT);
    drop(server_handle);
}
