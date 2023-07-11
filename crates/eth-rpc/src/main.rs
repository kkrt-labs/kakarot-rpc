use dotenv::dotenv;
use eyre::Result;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, SequencerGatewayProvider};
use tracing_subscriber::util::SubscriberInitExt;

enum KakarotProvider {
    JsonRpcClient(JsonRpcClient<HttpTransport>),
    SequencerGatewayProvider(SequencerGatewayProvider),
}

enum KakarotClientType {
    RpcClient(KakarotClient<JsonRpcClient<HttpTransport>>),
    GatewayClient(KakarotClient<SequencerGatewayProvider>),
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // Environment variables are safe to use after this

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_config = StarknetConfig::from_env()?;

    let rpc_config = RPCConfig::from_env()?;

    let provider: KakarotProvider = match &starknet_config.network {
        Network::Madara | Network::Katana => {
            KakarotProvider::JsonRpcClient(JsonRpcClientBuilder::with_http(&starknet_config).unwrap().build())
        }

        Network::Goerli1 => {
            KakarotProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_goerli())
        }

        Network::Goerli2 => {
            KakarotProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_goerli_2())
        }

        Network::Mainnet => {
            KakarotProvider::SequencerGatewayProvider(SequencerGatewayProvider::starknet_alpha_mainnet())
        }

        Network::ProviderUrl(url) => {
            KakarotProvider::JsonRpcClient(JsonRpcClientBuilder::new(HttpTransport::new(url.clone())).build())
        }
    };

    let kakarot_client = match provider {
        KakarotProvider::JsonRpcClient(provider) => {
            KakarotClientType::RpcClient(KakarotClient::new(starknet_config, provider))
        }
        KakarotProvider::SequencerGatewayProvider(provider) => {
            KakarotClientType::GatewayClient(KakarotClient::new(starknet_config, provider))
        }
    };

    let (server_addr, server_handle) = match kakarot_client {
        KakarotClientType::GatewayClient(kakarot_client) => run_server(Box::new(kakarot_client), rpc_config).await?,
        KakarotClientType::RpcClient(kakarot_client) => run_server(Box::new(kakarot_client), rpc_config).await?,
    };

    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
