use dotenv::dotenv;
use eyre::Result;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::StarknetConfig;
use kakarot_rpc_core::client::KakarotClient;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    // Environment variables are safe to use after this

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?
        .add_directive("jsonrpsee[method_call{name = \"eth_chainId\"}]=trace".parse()?);
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_cfg = StarknetConfig::from_env()?;
    let rpc_cfg = RPCConfig::from_env()?;
    let kakarot_client = KakarotClient::new(starknet_cfg)?;

    let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_cfg).await?;
    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
