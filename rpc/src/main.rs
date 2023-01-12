use eyre::{eyre, Result};
use futures::future::pending;
use kakarot_rpc::run_server;
use tracing_subscriber::util::SubscriberInitExt;
use kakarot_rpc_core::lightclient::StarknetClient;


#[tokio::main]
async fn main() -> Result<()> {

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()?
    .add_directive("jsonrpsee[method_call{name = \"eth_chainId\"}]=trace".parse()?);
    tracing_subscriber::FmtSubscriber::builder().with_env_filter(filter).finish().try_init()?;

    let starknet_rpc = std::env::var("STARKNET_RPC_URL")
    .map_err(|_| eyre!("Missing mandatory environment variable: STARKNET_RPC_URL"))?;

    let starknet_lightclient = StarknetClient::new(&starknet_rpc)?;

    let server_addr = run_server(starknet_lightclient).await?;
    let url = format!("http://{}", server_addr);
    
    println!("{url}");

    pending::<()>().await;

    Ok(())
}

