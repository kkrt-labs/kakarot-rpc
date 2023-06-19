// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, SocketAddr};
pub mod eth_rpc;
use eth_api::EthApiServer;
use eth_rpc::KakarotEthRpc;
pub mod eth_api;
use eyre::Result;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use kakarot_rpc_core::client::client_api::KakarotClient;
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
    starknet_client: Box<dyn KakarotClient>,
) -> Result<(SocketAddr, ServerHandle), RpcError> {
    let socket_addr =
        std::env::var("KAKAROT_HTTP_RPC_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3030".to_owned());

    let server = ServerBuilder::default()
        .build(socket_addr.parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;

    let rpc_calls = KakarotEthRpc::new(starknet_client);
    let handle = server.start(rpc_calls.into_rpc())?;

    Ok((addr, handle))
}
