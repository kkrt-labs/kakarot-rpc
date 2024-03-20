// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, Ipv4Addr, SocketAddr};

use config::RPCConfig;
pub mod api;
pub mod config;
pub mod middleware;
pub mod rpc;
pub mod servers;

use crate::eth_rpc::middleware::metrics::RpcMetrics;
use crate::eth_rpc::middleware::MetricsLayer;
use crate::prometheus_handler::init_prometheus;
use eyre::Result;
use jsonrpsee::server::middleware::http::{InvalidPath, ProxyGetRequestLayer};
use jsonrpsee::server::{RpcServiceBuilder, ServerBuilder, ServerHandle};
use jsonrpsee::RpcModule;
use prometheus::Registry;
use thiserror::Error;

use tower_http::cors::{Any, CorsLayer};

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    IoError(#[from] std::io::Error),
    #[error(transparent)]
    ParseError(#[from] AddrParseError),
    #[error(transparent)]
    JsonRpcError(#[from] InvalidPath),
    #[error(transparent)]
    PrometheusHandlerError(#[from] crate::prometheus_handler::Error),
    #[error(transparent)]
    PrometheusError(#[from] prometheus::Error),
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

    let registry = Registry::new();
    let metrics = RpcMetrics::new(Some(&registry))?.map(|m| MetricsLayer::new(m, "http"));
    tokio::spawn(async move {
        let _ = init_prometheus(
            SocketAddr::new(
                std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                get_env_or_default("PROMETHEUS_PORT", "9615").parse().unwrap(),
            ),
            registry.clone(),
        )
        .await;
    });
    let rpc_middleware = RpcServiceBuilder::new().option_layer(metrics);

    let server = ServerBuilder::default()
        .max_connections(get_env_or_default("RPC_MAX_CONNECTIONS", "100").parse().unwrap())
        .set_http_middleware(http_middleware)
        .set_rpc_middleware(rpc_middleware)
        .build(socket_addr.parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;
    let handle = server.start(kakarot_rpc_module);

    Ok((addr, handle))
}

fn get_env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}
