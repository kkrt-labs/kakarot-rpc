// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, SocketAddr};

use config::RPCConfig;
pub mod api;
pub mod config;
pub mod rpc;
pub mod servers;
use eyre::Result;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use jsonrpsee::RpcModule;
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
    kakarot_rpc_module: RpcModule<()>,
    rpc_config: RPCConfig,
) -> Result<(SocketAddr, ServerHandle), RpcError> {
    let RPCConfig { socket_addr } = rpc_config;

    let server = ServerBuilder::default().build(socket_addr.parse::<SocketAddr>()?).await?;

    let addr = server.local_addr()?;

    let handle = server.start(kakarot_rpc_module)?;

    Ok((addr, handle))
}
