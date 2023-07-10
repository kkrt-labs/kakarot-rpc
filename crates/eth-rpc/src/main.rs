use dotenv::dotenv;
use eyre::Result;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use starknet::providers::SequencerGatewayProvider;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // Environment variables are safe to use after this

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?;
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_config = StarknetConfig::from_env()?;

    let rpc_config = RPCConfig::from_env()?;

    // This match creates redundancy but it appears KakarotClient<P> complains about the
    // the possibility of P being either a JsonRpcClient or a SequencerGatewayProvider, but not being
    // known at compile time.
    match starknet_config.network {
        Network::Madara | Network::Katana => {
            let provider = JsonRpcClientBuilder::with_http(&starknet_config).unwrap().build();
            let kakarot_client = KakarotClient::new(starknet_config, provider);

            let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await?;
            let url = format!("http://{server_addr}");

            println!("RPC Server running on {url}...");

            server_handle.stopped().await;
        }
        Network::Goerli1 => {
            let provider = SequencerGatewayProvider::starknet_alpha_goerli();
            let kakarot_client = KakarotClient::new(starknet_config, provider);

            let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await?;
            let url = format!("http://{server_addr}");

            println!("RPC Server running on {url}...");

            server_handle.stopped().await;
        }
        Network::Goerli2 => {
            let provider = SequencerGatewayProvider::starknet_alpha_goerli_2();
            let kakarot_client = KakarotClient::new(starknet_config, provider);

            let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await?;
            let url = format!("http://{server_addr}");

            println!("RPC Server running on {url}...");

            server_handle.stopped().await;
        }
        Network::Mainnet => {
            let provider = SequencerGatewayProvider::starknet_alpha_mainnet();
            let kakarot_client = KakarotClient::new(starknet_config, provider);

            let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await?;
            let url = format!("http://{server_addr}");

            println!("RPC Server running on {url}...");

            server_handle.stopped().await;
        }
        Network::Mock => {
            return Err(eyre::eyre!("Cannot run RPC server with mock network"));
        }
    }

    Ok(())
}
