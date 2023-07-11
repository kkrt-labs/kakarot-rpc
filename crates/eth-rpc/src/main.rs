use std::sync::Arc;

use dotenv::dotenv;
use eyre::Result;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::rpc::KakarotRpcModuleBuilder;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
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

    let starknet_config = StarknetConfig::from_env()?;

    let rpc_config = RPCConfig::from_env()?;

    let starknet_provider: StarknetProvider = match &starknet_config.network {
        Network::Madara | Network::Katana => {
            StarknetProvider::JsonRpcClient(JsonRpcClientBuilder::with_http(&starknet_config).unwrap().build())
        }

        Network::Goerli1 => {
            StarknetProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_goerli())
        }

        Network::Goerli2 => {
            StarknetProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_goerli_2())
        }

        Network::Mainnet => {
            StarknetProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_mainnet())
        }

        Network::ProviderUrl(url) => {
            StarknetProvider::JsonRpcClient(JsonRpcClientBuilder::new(HttpTransport::new(url.clone())).build())
        }
    };

    let kakarot_rpc_module = match starknet_provider {
        StarknetProvider::JsonRpcClient(starknet_provider) => {
            let kakarot_client = Arc::new(KakarotClient::new(starknet_config, starknet_provider));
            KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()
        }
        StarknetProvider::SequencerGatewayProvider(starknet_provider) => {
            let kakarot_client = Arc::new(KakarotClient::new(starknet_config, starknet_provider));
            KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()
        }
    }?;

    let (server_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
