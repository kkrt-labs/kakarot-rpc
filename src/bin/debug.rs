use std::collections::HashMap;

use clap::Parser;
use ethers::prelude::LocalWallet;
use ethers::signers::Signer;
use eyre::OptionExt;
use hex::FromHex;
use kakarot_rpc::models::transaction::rpc_transaction_to_primitive;
use lazy_static::lazy_static;
use reth_primitives::{sign_message, Address, TransactionSigned, B256};
use reth_rpc_types::{Transaction, TransactionReceipt};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use tracing_subscriber::{filter, FmtSubscriber};

const KAKAROT_SEPOLIA_RPC_URL: &str = "https://kkrt-rpc-kakarot-dev.karnot.xyz/";
const LOCAL_RPC_URL: &str = "http://localhost:3030";

lazy_static! {
    static ref SECRET: B256 = B256::from_hex("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .expect("failed to parse secret");
    static ref SIGNER: Address = Address::from_slice(
        LocalWallet::from_bytes(SECRET.as_slice()).expect("failed to create signer").address().as_bytes()
    );
}

#[derive(Debug, Parser)]
pub struct DebugCommand {
    /// Transaction hash
    #[arg(short, long)]
    transaction_hash: B256,
    /// Create transaction hash for the
    /// contract under debug
    #[arg(short, long)]
    create_transaction_hash: B256,
}

#[tokio::main]
async fn main() {
    let filter = filter::EnvFilter::new("info");
    let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let debug_args = DebugCommand::parse();

    let reqwest_client = reqwest::Client::new();
    let mut debug_tx: Transaction = eth_tx_getter(
        &reqwest_client,
        KAKAROT_SEPOLIA_RPC_URL,
        "eth_getTransactionByHash",
        debug_args.transaction_hash,
    )
    .await
    .expect("failed to get debug transaction by hash");

    let create_tx = eth_tx_getter(
        &reqwest_client,
        KAKAROT_SEPOLIA_RPC_URL,
        "eth_getTransactionByHash",
        debug_args.create_transaction_hash,
    )
    .await
    .expect("failed to get create transaction by hash");

    let create_tx = rpc_tx_to_signed_tx(create_tx, 0).expect("failed to convert create transaction");
    let create_tx_hash =
        eth_send_transaction(&reqwest_client, create_tx).await.expect("failed to send create transaction");
    tracing::info!("create tx hash: {:?}", create_tx_hash);

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let receipt: TransactionReceipt =
        eth_tx_getter(&reqwest_client, LOCAL_RPC_URL, "eth_getTransactionReceipt", create_tx_hash)
            .await
            .expect("failed to get create transaction receipt");
    tracing::info!("create tx receipt: {:?}", receipt);

    debug_tx.to = receipt.contract_address;
    let debug_tx = rpc_tx_to_signed_tx(debug_tx, 1).expect("failed to convert debug transaction");

    let debug_tx_hash =
        eth_send_transaction(&reqwest_client, debug_tx).await.expect("failed to send debug transaction");
    tracing::info!("debug transaction hash{:?}", debug_tx_hash);

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let receipt: TransactionReceipt =
        eth_tx_getter(&reqwest_client, LOCAL_RPC_URL, "eth_getTransactionReceipt", debug_tx_hash)
            .await
            .expect("failed to get debug transaction receipt");
    tracing::info!("debug tx receipt: {:?}", receipt);
}

fn rpc_tx_to_signed_tx(transaction: Transaction, nonce: u64) -> Result<TransactionSigned, eyre::Error> {
    let mut transaction = rpc_transaction_to_primitive(transaction)?;
    transaction.set_nonce(nonce);
    match transaction {
        reth_primitives::Transaction::Legacy(ref mut tx) => tx.gas_limit = tx.gas_limit * 2,
        reth_primitives::Transaction::Eip1559(ref mut tx) => tx.gas_limit = tx.gas_limit * 2,
        reth_primitives::Transaction::Eip2930(ref mut tx) => tx.gas_limit = tx.gas_limit * 2,
        _ => unreachable!("unexpected transaction type"),
    };

    let tx_hash = transaction.signature_hash();
    let signature = sign_message(*SECRET, tx_hash)?;

    Ok(TransactionSigned::from_transaction_and_signature(transaction, signature))
}

async fn eth_send_transaction(client: &reqwest::Client, tx: TransactionSigned) -> Result<B256, eyre::Error> {
    let res = client
        .post(LOCAL_RPC_URL)
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"eth_sendRawTransaction",
                    "params":[format!("{:064x}", tx.envelope_encoded())],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await?;
    let res = res.text().await?;
    let res: HashMap<String, Value> = serde_json::from_str(&res)?;

    Ok(serde_json::from_value(res.get("result").ok_or_eyre("missing result")?.clone())?)
}

async fn eth_tx_getter<T: DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
    method: &str,
    hash: B256,
) -> Result<T, eyre::Error> {
    let res = client
        .post(url)
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":method,
                    "params":[format!("0x{:064x}", hash)],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await?;
    let res = res.text().await?;
    let res: HashMap<String, Value> = serde_json::from_str(&res)?;

    Ok(serde_json::from_value(res.get("result").ok_or_eyre("missing result")?.clone())?)
}
