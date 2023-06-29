// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, SocketAddr};
pub mod eth_rpc;
use config::RPCConfig;
use eth_api::EthApiServer;
use eth_rpc::KakarotEthRpc;
pub mod config;
pub mod eth_api;
use eyre::Result;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use kakarot_rpc_core::client::client_api::KakarotEthApi;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    JsonRpcServerError(#[from] jsonrpsee::core::Error),
    #[error(transparent)]
    ParseError(#[from] AddrParseError),
}

/// # Errors
///
/// Will return `Err` if an error occurs when running the `ServerBuilder` start fails.
pub async fn run_server(
    kakarot_client: Box<dyn KakarotEthApi>,
    rpc_config: RPCConfig,
) -> Result<(SocketAddr, ServerHandle), RpcError> {
    let RPCConfig { socket_addr } = rpc_config;

    let server = ServerBuilder::default().build(socket_addr.parse::<SocketAddr>()?).await?;

    let addr = server.local_addr()?;

    let rpc_calls = KakarotEthRpc::new(kakarot_client);
    let handle = server.start(rpc_calls.into_rpc())?;

    Ok((addr, handle))
}
