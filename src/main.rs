use std::env::var;
use std::sync::Arc;

use dotenvy::dotenv;
use eyre::Result;
use kakarot_rpc::config::{JsonRpcClientBuilder, KakarotRpcConfig, Network, SequencerGatewayProviderBuilder};
use kakarot_rpc::eth_provider::database::Database;
use kakarot_rpc::eth_provider::pending_pool::start_retry_service;
use kakarot_rpc::eth_provider::provider::EthDataProvider;
use kakarot_rpc::eth_rpc::config::RPCConfig;
use kakarot_rpc::eth_rpc::rpc::KakarotRpcModuleBuilder;
use kakarot_rpc::eth_rpc::run_server;
use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, SequencerGatewayProvider};
use tracing_subscriber::util::SubscriberInitExt;

enum StarknetProvider {
    JsonRpcClient(JsonRpcClient<HttpTransport>),
    SequencerGatewayProvider(SequencerGatewayProvider),
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // Environment variables are safe to use after this
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_config = KakarotRpcConfig::from_env()?;

    let rpc_config = RPCConfig::from_env()?;

    let starknet_provider = match &starknet_config.network {
        Network::Madara | Network::Katana | Network::Sharingan => {
            StarknetProvider::JsonRpcClient(JsonRpcClientBuilder::with_http(&starknet_config).unwrap().build())
        }
        Network::JsonRpcProvider(url) => {
            StarknetProvider::JsonRpcClient(JsonRpcClientBuilder::new(HttpTransport::new(url.clone())).build())
        }
        _ => StarknetProvider::SequencerGatewayProvider(
            SequencerGatewayProviderBuilder::new(&starknet_config.network).build(),
        ),
    };

    let db_client =
        mongodb::Client::with_uri_str(var("MONGO_CONNECTION_STRING").expect("Missing MONGO_CONNECTION_STRING .env"))
            .await?;
    let db = Database::new(db_client.database_with_options(
        &var("MONGO_DATABASE_NAME").expect("Missing MONGO_DATABASE_NAME from .env"),
        DatabaseOptions::builder().read_concern(ReadConcern::MAJORITY).write_concern(WriteConcern::MAJORITY).build(),
    ));

    // Get the deployer nonce and set the value in the DEPLOY_WALLET_NONCE
    #[cfg(feature = "hive")]
    {
        use kakarot_rpc::eth_provider::constant::{CHAIN_ID, DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
        use starknet::accounts::ConnectedAccount;
        use starknet::providers::Provider as _;
        use starknet_crypto::FieldElement;

        let provider = JsonRpcClient::new(HttpTransport::new(
            starknet_config.network.provider_url().expect("Incorrect provider URL"),
        ));
        let chain_id = provider.chain_id().await?;
        let chain_id: u64 = (FieldElement::from(u64::MAX) & chain_id).try_into()?;

        CHAIN_ID.set(chain_id.into()).expect("Failed to set chain id");

        let deployer_nonce = DEPLOY_WALLET.get_nonce().await?;
        let mut nonce = DEPLOY_WALLET_NONCE.lock().await;
        *nonce = deployer_nonce;
    }

    let kakarot_rpc_module = match starknet_provider {
        StarknetProvider::JsonRpcClient(starknet_provider) => {
            let starknet_provider = Arc::new(starknet_provider);
            let eth_provider = EthDataProvider::new(db.clone(), starknet_provider).await?;
            tokio::spawn(start_retry_service(eth_provider.clone()));
            KakarotRpcModuleBuilder::new(eth_provider).rpc_module()?
        }
        StarknetProvider::SequencerGatewayProvider(starknet_provider) => {
            let starknet_provider = Arc::new(starknet_provider);
            let eth_provider = EthDataProvider::new(db.clone(), starknet_provider).await?;
            tokio::spawn(start_retry_service(eth_provider.clone()));
            KakarotRpcModuleBuilder::new(eth_provider).rpc_module()?
        }
    };

    let (socket_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    let url = format!("http://{}", socket_addr);

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
