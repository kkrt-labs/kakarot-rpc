#![cfg(feature = "testing")]
use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::test_utils::eoa::Eoa;
use kakarot_rpc::test_utils::evm_contract::{EvmContract, KakarotEvmContract};
use kakarot_rpc::test_utils::fixtures::{plain_opcodes, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use reth_primitives::{U256, U8};
use reth_rpc_types::trace::parity::LocalizedTransactionTrace;
use reth_rpc_types::Signature;
use rstest::*;
use serde_json::{json, Value};

const TRACING_BLOCK_NUMBER: u64 = 0x2;
const TRANSACTIONS_COUNT: usize = 2;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_trace_block(#[future] plain_opcodes: (Katana, KakarotEvmContract), _setup: ()) {
    let katana = plain_opcodes.0;
    let plain_opcodes = plain_opcodes.1;
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get eoa address");
    let nonce: u64 = eoa.nonce().await.expect("Failed to get nonce").to();
    let chain_id = eoa.eth_provider().chain_id().await.expect("Failed to get chain id").unwrap_or_default();

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
                nonce + i as u64,
                0,
                chain_id.to(),
                max_fee_per_gas,
                max_priority_fee_per_gas,
            )
            .expect("Failed to prepare call transaction");
        // Sign the transaction and convert it to a RPC transaction.
        let tx_signed = eoa.sign_transaction(tx.clone()).expect("Failed to sign transaction");
        let tx = reth_rpc_types::Transaction {
            transaction_type: Some(U8::from(2)),
            nonce: tx.nonce(),
            hash: tx_signed.hash(),
            to: tx.to(),
            from: eoa_address,
            block_number: Some(U256::from(TRACING_BLOCK_NUMBER)),
            chain_id: tx.chain_id(),
            gas: U256::from(tx.gas_limit()),
            input: tx.input().clone(),
            signature: Some(Signature {
                r: tx_signed.signature().r,
                s: tx_signed.signature().s,
                v: U256::from(tx_signed.signature().v(Some(chain_id.to()))),
                y_parity: Some(reth_rpc_types::Parity(tx_signed.signature().odd_y_parity)),
            }),
            max_fee_per_gas: Some(U256::from(max_fee_per_gas)),
            gas_price: Some(U256::from(max_fee_per_gas)),
            max_priority_fee_per_gas: Some(U256::from(max_priority_fee_per_gas)),
            value: tx.value(),
            ..Default::default()
        };
        txs.push(tx);
    }
    katana.add_transactions_with_header_to_database(txs, TRACING_BLOCK_NUMBER).await;

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

    drop(server_handle);
}
