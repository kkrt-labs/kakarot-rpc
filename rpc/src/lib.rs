// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::SocketAddr;
mod eth_rpc;
use eth_rpc::{EthApiServer, KakarotEthRpc};
use eyre::Result;
use jsonrpsee::server::ServerBuilder;
use kakarot_rpc_core::lightclient::StarknetClient;

pub async fn run_server(starknet_client: StarknetClient) -> Result<SocketAddr> {
    let server = ServerBuilder::default()
        .build("127.0.0.1:03030".parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr();

    let rpc_calls = KakarotEthRpc::new(starknet_client);
    let handle = server.start(rpc_calls.into_rpc())?;
    tokio::spawn(handle.stopped());
    Ok(addr.unwrap())
}
