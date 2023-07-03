use dotenv::dotenv;
use eyre::Result;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // Environment variables are safe to use after this

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?
        .add_directive("jsonrpsee[method_call{name = \"eth_chainId\"}]=trace".parse()?);
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_config = StarknetConfig::from_env()?;
    let rpc_config = RPCConfig::from_env()?;

    let provider = JsonRpcClientBuilder::with_http(&starknet_config).unwrap().build();
    let kakarot_client = KakarotClient::new(starknet_config, provider)?;

    let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await?;
    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
