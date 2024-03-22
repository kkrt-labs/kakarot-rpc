// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use config::RPCConfig;
use eyre::Result;
use jsonrpsee::{
    server::{
        middleware::http::{InvalidPath, ProxyGetRequestLayer},
        ServerBuilder, ServerHandle,
    },
    RpcModule,
};
use std::net::{AddrParseError, SocketAddr};
use thiserror::Error;
use tower_http::cors::{Any, CorsLayer};

pub mod api;
pub mod config;
pub mod rpc;
pub mod servers;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseError(#[from] AddrParseError),
    #[error(transparent)]
    JsonRpcError(#[from] InvalidPath),
}

/// # Errors
///
/// Will return `Err` if an error occurs when running the `ServerBuilder` start fails.
pub async fn run_server(
    kakarot_rpc_module: RpcModule<()>,
    rpc_config: RPCConfig,
) -> Result<(SocketAddr, ServerHandle), RpcError> {
    let RPCConfig { socket_addr } = rpc_config;

    let cors = CorsLayer::new().allow_methods(Any).allow_origin(Any).allow_headers(Any);

    let http_middleware =
        tower::ServiceBuilder::new().layer(ProxyGetRequestLayer::new("/health", "net_health")?).layer(cors);

    let server = ServerBuilder::default()
        .max_connections(std::env::var("RPC_MAX_CONNECTIONS").unwrap_or_else(|_| "100".to_string()).parse().unwrap())
        .set_http_middleware(http_middleware)
        .build(socket_addr.parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;

    let handle = server.start(kakarot_rpc_module);

    Ok((addr, handle))
}
