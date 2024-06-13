#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use ethers::abi::Tokenize;
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::test_utils::eoa::Eoa;
use kakarot_rpc::test_utils::evm_contract::{
    EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo,
};
use kakarot_rpc::test_utils::fixtures::{plain_opcodes, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use kakarot_rpc::test_utils::rpc::RawRpcParamsBuilder;
use reth_primitives::{Address, Bytes, B256, U256};
use reth_rpc_types::other::OtherFields;
use reth_rpc_types::trace::geth::{GethTrace, TraceResult};
use reth_rpc_types::trace::parity::{
    Action, CallAction, CallOutput, CallType, LocalizedTransactionTrace, TraceOutput, TransactionTrace,
};
use rstest::*;
use serde_json::{json, Value};
use starknet::core::types::MaybePendingBlockWithTxHashes;
use starknet::providers::Provider;

/// The block number on which tracing will be performed.
const TRACING_BLOCK_NUMBER: u64 = 0x3;
/// The amount of transactions to be traced.
const TRACING_TRANSACTIONS_COUNT: usize = 5;

/// Helper to create a header.
fn header(block_number: u64, hash: B256, parent_hash: B256, base_fee: u128) -> reth_rpc_types::Header {
    reth_rpc_types::Header {
        number: Some(block_number),
        hash: Some(hash),
        parent_hash,
        gas_limit: u128::from(u64::MAX),
        base_fee_per_gas: Some(base_fee),
        ..Default::default()
    }
}

/// Helper to set up the debug/tracing environment on Katana.
pub async fn tracing<T: Tokenize>(
    katana: &Katana,
    contract: &KakarotEvmContract,
    entry_point: &str,
    get_args: Box<dyn Fn(u64) -> T>,
) {
    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get eoa address");
    let nonce: u64 = eoa.nonce().await.expect("Failed to get nonce").to();
    let chain_id = eoa.eth_provider().chain_id().await.expect("Failed to get chain id").unwrap_or_default().to();

    // Push 10 RPC transactions into the database.
    let mut txs = Vec::with_capacity(TRACING_TRANSACTIONS_COUNT);
    let max_fee_per_gas = 10;
    let max_priority_fee_per_gas = 1;
    for i in 0..TRACING_TRANSACTIONS_COUNT {
        let tx = contract
            .prepare_call_transaction(
                entry_point,
                get_args(nonce + i as u64),
                &TransactionInfo::FeeMarketInfo(TxFeeMarketInfo {
                    common: TxCommonInfo { nonce: nonce + i as u64, value: 0, chain_id: Some(chain_id) },
                    max_fee_per_gas,
                    max_priority_fee_per_gas,
                }),
            )
            .expect("Failed to prepare call transaction");
        // Sign the transaction and convert it to a RPC transaction.
        let tx_signed = eoa.sign_transaction(tx.clone()).expect("Failed to sign transaction");
        let mut tx = reth_rpc_types::Transaction {
            transaction_type: Some(2),
            nonce: tx.nonce(),
            hash: tx_signed.hash(),
            to: tx.to(),
            from: eoa_address,
            block_number: Some(TRACING_BLOCK_NUMBER),
            chain_id: tx.chain_id(),
            gas: u128::from(tx.gas_limit()),
            input: tx.input().clone(),
            signature: Some(reth_rpc_types::Signature {
                r: tx_signed.signature().r,
                s: tx_signed.signature().s,
                v: U256::from(tx_signed.signature().odd_y_parity),
                y_parity: Some(reth_rpc_types::Parity(tx_signed.signature().odd_y_parity)),
            }),
            max_fee_per_gas: Some(max_fee_per_gas),
            gas_price: Some(max_fee_per_gas),
            max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
            value: tx.value(),
            access_list: Some(Default::default()),
            ..Default::default()
        };

        // Add an out of resources field to the last transaction.
        if i == TRACING_TRANSACTIONS_COUNT - 1 {
            let mut out_of_resources = std::collections::BTreeMap::new();
            out_of_resources.insert(String::from("isRunOutOfResources"), serde_json::Value::Bool(true));
            tx.other = OtherFields::new(out_of_resources);
        }

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
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| ())).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the trace_block RPC request.
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("trace_block").add_param(format!("0x{TRACING_BLOCK_NUMBER:016x}")).build())
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let traces: Option<Vec<LocalizedTransactionTrace>> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    // Assert that traces is not None, meaning the response contains some traces.
    assert!(traces.is_some());
    // We expect 3 traces per transaction: CALL, CREATE, and CALL.
    // Except for the last one which is out of resources.
    assert!(traces.clone().unwrap().len() == 3 * (TRACING_TRANSACTIONS_COUNT - 1) + 1);

    // Get the last trace from the trace vector, which is expected to be out of resources.
    let trace_vec = traces.unwrap();
    let out_of_resources_trace = trace_vec.last().unwrap();

    // Assert that the block number of the out-of-resources trace is equal to the expected TRACING_BLOCK_NUMBER.
    assert_eq!(out_of_resources_trace.clone().block_number, Some(TRACING_BLOCK_NUMBER));
    // Assert that the trace matches the expected default TransactionTrace.
    assert_eq!(
        out_of_resources_trace.trace,
        TransactionTrace {
            action: Action::Call(CallAction {
                from: Address::ZERO,
                call_type: CallType::Call,
                gas: Default::default(),
                input: Bytes::default(),
                to: Address::ZERO,
                value: U256::ZERO
            }),
            error: None,
            result: Some(TraceOutput::Call(CallOutput { gas_used: Default::default(), output: Bytes::default() })),
            subtraces: 0,
            trace_address: vec![],
        }
    );

    drop(server_handle);
}

async fn trace_block_by_number(port: u16) -> Vec<TraceResult> {
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{port}"))
        .header("Content-Type", "application/json")
        .body(
            RawRpcParamsBuilder::new("debug_traceBlockByNumber")
                .add_param(format!("0x{TRACING_BLOCK_NUMBER:016x}"))
                .add_param(json!({
                    "tracer": "callTracer",
                    "tracerConfig": {
                        "onlyTopCall": false
                    },
                    "timeout": "300s"
                }))
                .build(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");

    serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result")
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_debug_trace_block_by_number(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    // Setup the Kakarot RPC server.
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| ())).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the trace_block RPC request.
    let traces = trace_block_by_number(server_addr.port()).await;

    // We expect 1 trace per transaction given the formatting of the debug_traceBlockByNumber response.
    assert!(traces.len() == TRACING_TRANSACTIONS_COUNT);
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_debug_trace_transaction(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    // Setup the Kakarot RPC server.
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| ())).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Get the block in order to trace a transaction.
    let block = katana
        .eth_provider()
        .block_by_number(TRACING_BLOCK_NUMBER.into(), false)
        .await
        .expect("Failed to get block")
        .unwrap();
    let index = TRACING_TRANSACTIONS_COUNT - 2;
    let tx_hash = block.transactions.as_hashes().unwrap().get(index).unwrap();

    // Send the trace_block RPC request.
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            RawRpcParamsBuilder::new("debug_traceTransaction")
                .add_param(format!("0x{tx_hash:016x}"))
                .add_param(json!({
                    "tracer": "callTracer",
                    "tracerConfig": {
                        "onlyTopCall": false
                    },
                    "timeout": "300s"
                }))
                .build(),
        )
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let trace: GethTrace = serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    // Get the traces for the block
    let traces = trace_block_by_number(server_addr.port()).await;
    let reth_rpc_types::trace::geth::TraceResult::Success { result: expected_trace, .. } =
        traces.get(index).cloned().unwrap()
    else {
        panic!("Failed to get expected trace")
    };

    // Compare traces
    assert_eq!(expected_trace, trace);

    // Get the last trace from the trace vector, which is expected to be out of resources.
    let run_out_of_resource_trace = traces.last().unwrap();

    // Asser that the trace matches the expected default GethTrace for a transaction that runs out of resources.
    match run_out_of_resource_trace {
        TraceResult::Success { result, .. } => assert_eq!(
            *result,
            GethTrace::Default(reth_rpc_types::trace::geth::DefaultFrame { failed: true, ..Default::default() })
        ),
        TraceResult::Error { .. } => panic!("Expected a success trace result"),
    };

    drop(server_handle);
}
