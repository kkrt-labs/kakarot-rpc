use std::env::var;
use std::sync::Arc;

use dotenvy::dotenv;
use eyre::Result;
use kakarot_rpc::config::KakarotRpcConfig;
use kakarot_rpc::eth_provider::database::Database;
use kakarot_rpc::eth_provider::pending_pool::start_retry_service;
use kakarot_rpc::eth_provider::provider::EthDataProvider;
use kakarot_rpc::eth_rpc::config::RPCConfig;
use kakarot_rpc::eth_rpc::rpc::KakarotRpcModuleBuilder;
use kakarot_rpc::eth_rpc::run_server;
use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use tracing_subscriber::{filter, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    // Environment variables are safe to use after this
    dotenv().ok();

    let filter = format!("kakarot_rpc={}", std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()));
    let filter = filter::EnvFilter::new(filter);
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    // Load the configuration
    let starknet_config = KakarotRpcConfig::from_env()?;
    let rpc_config = RPCConfig::from_env()?;

    let starknet_provider = JsonRpcClient::new(HttpTransport::new(starknet_config.network_url));

    // Setup the database
    let db_client =
        mongodb::Client::with_uri_str(var("MONGO_CONNECTION_STRING").expect("Missing MONGO_CONNECTION_STRING .env"))
            .await?;
    let db = Database::new(db_client.database_with_options(
        &var("MONGO_DATABASE_NAME").expect("Missing MONGO_DATABASE_NAME from .env"),
        DatabaseOptions::builder().read_concern(ReadConcern::MAJORITY).write_concern(WriteConcern::MAJORITY).build(),
    ));

    // Setup hive
    #[cfg(feature = "hive")]
    setup_hive(&starknet_provider).await?;

    // Setup the retry service
    let eth_provider = EthDataProvider::new(db, Arc::new(starknet_provider)).await?;
    tokio::spawn(start_retry_service(eth_provider.clone()));
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(eth_provider).rpc_module()?;

    // Start the RPC server
    let (socket_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    let url = format!("http://{socket_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}

#[allow(clippy::significant_drop_tightening)]
#[cfg(feature = "hive")]
async fn setup_hive(starknet_provider: &JsonRpcClient<HttpTransport>) -> Result<()> {
    use kakarot_rpc::eth_provider::constant::{CHAIN_ID, DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
    use starknet::accounts::ConnectedAccount;
    use starknet::providers::Provider as _;
    use starknet_crypto::FieldElement;

    let chain_id = starknet_provider.chain_id().await?;
    let chain_id: u64 = (FieldElement::from(u64::MAX) & chain_id).try_into()?;

    CHAIN_ID.set(chain_id.into()).expect("Failed to set chain id");

    let deployer_nonce = DEPLOY_WALLET.get_nonce().await?;
    let mut nonce = DEPLOY_WALLET_NONCE.lock().await;
    *nonce = deployer_nonce;

    Ok(())
}
