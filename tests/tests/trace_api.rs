#![cfg(feature = "testing")]
use ethers::abi::{Token, Tokenize};
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::test_utils::eoa::Eoa;
use kakarot_rpc::test_utils::evm_contract::{
    EvmContract, KakarotEvmContract, TransactionInfo, TxCommonInfo, TxFeeMarketInfo, TxLegacyInfo,
};
use kakarot_rpc::test_utils::fixtures::{eip_3074_invoker, plain_opcodes, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use kakarot_rpc::test_utils::rpc::RawRpcParamsBuilder;
use reth_primitives::{Address, B256, U256};
use reth_rpc_types::trace::geth::{GethTrace, TraceResult};
use reth_rpc_types::trace::parity::LocalizedTransactionTrace;
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
        gas_limit: u64::MAX as u128,
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
        .body(RawRpcParamsBuilder::new("trace_block").add_param(format!("0x{:016x}", TRACING_BLOCK_NUMBER)).build())
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let traces: Option<Vec<LocalizedTransactionTrace>> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    assert!(traces.is_some());
    // We expect 3 traces per transaction: CALL, CREATE, and CALL.
    assert!(traces.unwrap().len() == 3 * TRACING_TRANSACTIONS_COUNT);
    drop(server_handle);
}

async fn trace_block_by_number(port: u16) -> Vec<TraceResult> {
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", port))
        .header("Content-Type", "application/json")
        .body(
            RawRpcParamsBuilder::new("debug_traceBlockByNumber")
                .add_param(format!("0x{:016x}", TRACING_BLOCK_NUMBER))
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
async fn test_trace_eip3074(#[future] eip_3074_invoker: (Katana, KakarotEvmContract, KakarotEvmContract), _setup: ()) {
    // Setup the Kakarot RPC server.
    let katana = eip_3074_invoker.0;
    let counter = eip_3074_invoker.1;
    let invoker = eip_3074_invoker.2;

    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get eoa address");

    let chain_id = katana.eth_provider().chain_id().await.expect("Failed to get chain id").unwrap_or_default().to();
    let invoker_address = Address::from_slice(&invoker.evm_address.to_bytes_be()[12..]);
    let commit = B256::default();

    let get_args = Box::new(move |nonce| {
        // Taken from https://github.com/paradigmxyz/alphanet/blob/87be409101bab9c8977b0e74edfb25334511a74c/crates/instructions/src/eip3074.rs#L129
        // Composes the message expected by the AUTH instruction in this format:
        // `keccak256(MAGIC || chainId || nonce || invokerAddress || commit)`
        fn compose_msg(chain_id: u64, nonce: u64, invoker_address: Address, commit: B256) -> B256 {
            let mut msg = [0u8; 129];
            // MAGIC constant is used for [EIP-3074](https://eips.ethereum.org/EIPS/eip-3074) signatures to prevent signature collisions with other signing formats.
            msg[0] = 0x4;
            msg[1..33].copy_from_slice(B256::left_padding_from(&chain_id.to_be_bytes()).as_slice());
            msg[33..65].copy_from_slice(B256::left_padding_from(&nonce.to_be_bytes()).as_slice());
            msg[65..97].copy_from_slice(B256::left_padding_from(invoker_address.as_slice()).as_slice());
            msg[97..].copy_from_slice(commit.as_slice());
            reth_primitives::keccak256(msg.as_slice())
        }
        // We use nonce + 1 because authority == sender.
        let msg = compose_msg(chain_id, nonce + 1, invoker_address, commit);
        let signature = eoa.sign_payload(msg).expect("Failed to sign message");
        let calldata = counter
            .prepare_call_transaction("inc", (), &TransactionInfo::LegacyInfo(TxLegacyInfo::default()))
            .expect("Failed to prepare call transaction")
            .input()
            .clone();

        (
            Token::Address(ethers::abi::Address::from_slice(eoa_address.as_slice())),
            Token::FixedBytes(commit.as_slice().to_vec()),
            Token::Uint(ethers::abi::Uint::from(signature.odd_y_parity as u8 + 27)),
            Token::FixedBytes(signature.r.to_be_bytes::<32>().to_vec()),
            Token::FixedBytes(signature.s.to_be_bytes::<32>().to_vec()),
            Token::Address(ethers::abi::Address::from_slice(&counter.evm_address.to_bytes_be()[12..])),
            Token::Bytes(calldata.to_vec()),
        )
    });

    // Set up the transactions for tracing. We call the sponsorCall entry point which should
    // auth the invoker as the sender and then call the inc entry point on the counter contract.
    tracing(&katana, &invoker, "sponsorCall", get_args).await;

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // Send the trace_block RPC request.
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("trace_block").add_param(format!("0x{:016x}", TRACING_BLOCK_NUMBER)).build())
        .send()
        .await
        .expect("Failed to call Debug RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let traces: Option<Vec<LocalizedTransactionTrace>> =
        serde_json::from_value(raw["result"].clone()).expect("Failed to deserialize result");

    assert!(traces.is_some());
    // We expect 2 traces per transaction: CALL and CALL.
    assert!(traces.unwrap().len() == 2 * TRACING_TRANSACTIONS_COUNT);
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
                .add_param(format!("0x{:016x}", tx_hash))
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
    let expected_trace =
        if let reth_rpc_types::trace::geth::TraceResult::Success { result, .. } = traces.get(index).cloned().unwrap() {
            result
        } else {
            panic!("Failed to get expected trace")
        };

    // Compare traces
    assert_eq!(expected_trace, trace);

    drop(server_handle);
}
