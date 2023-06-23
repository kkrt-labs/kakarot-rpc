use dotenv::dotenv;
use eyre::{eyre, Result};
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::KakarotClientWrapper;
use starknet::core::types::FieldElement;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let starknet_rpc = std::env::var("STARKNET_RPC_URL")
        .map_err(|_| eyre!("Missing mandatory environment variable: STARKNET_RPC_URL"))?;

    let kakarot_address = std::env::var("KAKAROT_ADDRESS")
        .map_err(|_| eyre!("Missing mandatory environment variable: KAKAROT_ADDRESS"))?;
    let kakarot_address = FieldElement::from_hex_be(&kakarot_address)
        .map_err(|_| eyre!("KAKAROT_ADDRESS should be provided as a hex string, got {kakarot_address}"))?;

    let proxy_account_class_hash = std::env::var("PROXY_ACCOUNT_CLASS_HASH")
        .map_err(|_| eyre!("Missing mandatory environment variable: PROXY_ACCOUNT_CLASS_HASH"))?;
    let proxy_account_class_hash = FieldElement::from_hex_be(&proxy_account_class_hash).map_err(|_| {
        eyre!("PROXY_ACCOUNT_CLASS_HASH should be provided as a hex string, got {proxy_account_class_hash}")
    })?;

    let kakarot_client = KakarotClientWrapper::new(&starknet_rpc, kakarot_address, proxy_account_class_hash)?;

    let (server_addr, server_handle) = run_server(Box::new(kakarot_client)).await?;
    let url = format!("http://{server_addr}");

    println!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}
