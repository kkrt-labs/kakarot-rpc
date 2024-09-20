#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use alloy_dyn_abi::DynSolValue;
use kakarot_rpc::{
    providers::eth_provider::{BlockProvider, ChainProvider},
    test_utils::{
        eoa::Eoa,
        evm_contract::{EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo},
        fixtures::{plain_opcodes, setup},
        katana::Katana,
    },
    tracing::builder::TracerBuilder,
};
use reth_primitives::{Address, Bytes, B256, U256};
use reth_rpc_types::{
    trace::{
        geth::{GethDebugTracingOptions, GethTrace, TraceResult},
        parity::{Action, CallAction, CallOutput, CallType, TraceOutput, TransactionTrace},
    },
    OtherFields, WithOtherFields,
};
use revm_inspectors::tracing::TracingInspectorConfig;
use rstest::*;
use serde_json::json;
use starknet::{core::types::MaybePendingBlockWithTxHashes, providers::Provider};
use std::sync::Arc;

/// The block number on which tracing will be performed.
const TRACING_BLOCK_NUMBER: u64 = 0x3;
/// The amount of transactions to be traced.
const TRACING_TRANSACTIONS_COUNT: usize = 5;

/// Helper to create a header.
fn header(block_number: u64, hash: B256, parent_hash: B256, base_fee: u128) -> reth_rpc_types::Header {
    reth_rpc_types::Header {
        number: block_number,
        hash,
        parent_hash,
        gas_limit: u128::from(u64::MAX),
        base_fee_per_gas: Some(base_fee),
        ..Default::default()
    }
}

/// Helper to set up the debug/tracing environment on Katana.
pub async fn tracing(
    katana: &Katana,
    contract: &KakarotEvmContract,
    entry_point: &str,
    get_args: Box<dyn Fn(u64) -> Vec<DynSolValue>>,
) {
    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get eoa address");
    let nonce: u64 = eoa.nonce().await.expect("Failed to get nonce").to();
    let chain_id =
        eoa.eth_client().eth_provider().chain_id().await.expect("Failed to get chain id").unwrap_or_default().to();

    // Push 10 RPC transactions into the database.
    let mut txs = Vec::with_capacity(TRACING_TRANSACTIONS_COUNT);
    let max_fee_per_gas = 10;
    let max_priority_fee_per_gas = 1;
    for i in 0..TRACING_TRANSACTIONS_COUNT {
        let tx = contract
            .prepare_call_transaction(
                entry_point,
                &get_args(nonce + i as u64),
                &TransactionInfo::FeeMarketInfo(TxFeeMarketInfo {
                    common: TxCommonInfo { nonce: nonce + i as u64, value: 0, chain_id: Some(chain_id) },
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

        let mut tx = WithOtherFields::new(tx);

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
#[ignore = "failing because of relayer change"]
async fn test_trace_block(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| vec![])).await;

    // Get the Ethereum provider from the Katana instance.
    let eth_provider = katana.eth_provider();

    // Create a new TracerBuilder instance.
    let tracer_builder_block =
        TracerBuilder::new(Arc::new(&eth_provider)).await.expect("Failed to create tracer_builder_block");
    let tracer = tracer_builder_block
        .with_block_id(TRACING_BLOCK_NUMBER.into())
        .await
        .expect("Failed to set block number")
        .with_tracing_options(TracingInspectorConfig::default_parity().into())
        .build()
        .expect("Failed to build block_trace");
    // Trace the block and get the block traces.
    let block_traces = tracer.trace_block().expect("Failed to trace block");

    // Assert that traces is not None, meaning the response contains some traces.
    assert!(block_traces.is_some());

    let trace_vec = block_traces.unwrap_or_default();
    // We expect 3 traces per transaction: CALL, CREATE, and CALL.
    // Except for the last one which is out of resources.
    assert!(trace_vec.len() == 3 * (TRACING_TRANSACTIONS_COUNT - 1) + 1);

    // Get the last trace from the trace vector, which is expected to be out of resources.
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
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "failing because of relayer change"]
async fn test_debug_trace_block_by_number(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| vec![])).await;

    // Define tracing options for the geth debug tracer.
    let opts: GethDebugTracingOptions = serde_json::from_value(json!({
        "tracer": "callTracer",
        "tracerConfig": {
            "onlyTopCall": false
        },
        "timeout": "300s"
    }))
    .expect("Failed to deserialize tracing options");

    // Get the Ethereum provider from the Katana instance.
    let eth_provider = katana.eth_provider();
    // Create a new TracerBuilder instance.
    let tracer_builder_block =
        TracerBuilder::new(Arc::new(&eth_provider)).await.expect("Failed to create tracer_builder_block");

    // Get the traces for the block
    let block_trace = tracer_builder_block
        .with_block_id(TRACING_BLOCK_NUMBER.into())
        .await
        .expect("Failed to set block number")
        .with_tracing_options(kakarot_rpc::tracing::builder::TracingOptions::Geth(opts.clone()))
        .build()
        .expect("Failed to build block_trace");

    // Trace the block and get the block traces.
    let block_traces = block_trace.debug_block().expect("Failed to trace block by number");

    // We expect 1 trace per transaction given the formatting of the debug_traceBlockByNumber response.
    assert!(block_traces.len() == TRACING_TRANSACTIONS_COUNT);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[ignore = "failing because of relayer change"]
async fn test_debug_trace_transaction(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    tracing(&katana, &plain_opcodes, "createCounterAndInvoke", Box::new(|_| vec![])).await;

    // Get the block in order to trace a transaction.
    let block = katana
        .eth_provider()
        .block_by_number(TRACING_BLOCK_NUMBER.into(), false)
        .await
        .expect("Failed to get block")
        .unwrap();

    let index = TRACING_TRANSACTIONS_COUNT - 2;
    let tx_hash = block.transactions.as_hashes().unwrap().get(index).unwrap();

    let opts: GethDebugTracingOptions = serde_json::from_value(json!({
        "tracer": "callTracer",
        "tracerConfig": {
            "onlyTopCall": false
        },
        "timeout": "300s"
    }))
    .expect("Failed to deserialize tracing options");

    // Get the Ethereum provider from the Katana instance.
    let eth_provider = katana.eth_provider();
    // Create a TracerBuilder instance
    let tracer_builder = TracerBuilder::new(Arc::new(&eth_provider)).await.expect("Failed to create tracer_builder");

    // Get the traces for the tx.
    let trace_with_tx_hash = tracer_builder
        .clone()
        .with_transaction_hash(*tx_hash)
        .await
        .expect("Failed to set transaction hash")
        .with_tracing_options(kakarot_rpc::tracing::builder::TracingOptions::Geth(opts.clone()))
        .build()
        .expect("Failed to build trace_with_tx_hash");
    let trace = trace_with_tx_hash.debug_transaction(*tx_hash).expect("Failed to trace transaction");

    // Get the traces for the block
    let block_trace = tracer_builder
        .clone()
        .with_block_id(TRACING_BLOCK_NUMBER.into())
        .await
        .expect("Failed to set block number")
        .with_tracing_options(kakarot_rpc::tracing::builder::TracingOptions::Geth(opts.clone()))
        .build()
        .expect("Failed to build block_trace");
    let block_traces = block_trace.debug_block().expect("Failed to trace block by number");
    let reth_rpc_types::trace::geth::TraceResult::Success { result: expected_trace, .. } =
        block_traces.get(index).cloned().unwrap()
    else {
        panic!("Failed to get expected trace")
    };

    // Compare traces
    assert_eq!(expected_trace, trace);

    // Get the last trace from the trace vector, which is expected to be out of resources.
    let run_out_of_resource_trace = block_traces.last().unwrap();

    // Asser that the trace matches the expected default GethTrace for a transaction that runs out of resources.
    match run_out_of_resource_trace {
        TraceResult::Success { result, .. } => assert_eq!(
            result.clone(),
            GethTrace::Default(reth_rpc_types::trace::geth::DefaultFrame { failed: true, ..Default::default() })
        ),
        TraceResult::Error { .. } => panic!("Expected a success trace result"),
    };
}
