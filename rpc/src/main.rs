use eyre::Result;
use kakarot_rpc::run_server;

#[tokio::main]
async fn main() -> Result<()> {
    let (server_addr, server_handle) = run_server().await?;
    println!("{server_addr}");

    server_handle.stopped().await;
    Ok(())
}
