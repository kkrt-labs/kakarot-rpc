use dotenv::dotenv;
use eyre::{eyre, Result};
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::StarknetClientImpl;
use tracing_subscriber::util::SubscriberInitExt;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?
        .add_directive("jsonrpsee[method_call{name = \"eth_chainId\"}]=trace".parse()?);
    tracing_subscriber::FmtSubscriber::builder()
        .with_env_filter(filter)
        .finish()
        .try_init()?;

    let starknet_rpc = std::env::var("STARKNET_RPC_URL")
        .map_err(|_| eyre!("Missing mandatory environment variable: STARKNET_RPC_URL"))?;

    let starknet_lightclient = StarknetClientImpl::new(&starknet_rpc)?;

    let (server_addr, server_handle) = run_server(Box::new(starknet_lightclient)).await?;
    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
