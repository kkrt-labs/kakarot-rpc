// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.

/// Contains modules related to API functionality.
pub mod api;

/// Contains modules related to configuration settings.
pub mod config;

/// Contains modules related to middleware.
pub mod middleware;

/// Contains modules related to RPC (Remote Procedure Call).
pub mod rpc;

/// Contains modules related to server implementations.
pub mod servers;

use crate::eth_rpc::middleware::metrics::RpcMetrics;
use crate::eth_rpc::middleware::MetricsLayer;
use crate::prometheus_handler::init_prometheus;
use config::RPCConfig;
use eyre::Result;
use jsonrpsee::server::middleware::http::{InvalidPath, ProxyGetRequestLayer};
use jsonrpsee::server::{RpcServiceBuilder, ServerBuilder, ServerHandle};
use jsonrpsee::RpcModule;
use prometheus::Registry;
use std::net::{AddrParseError, Ipv4Addr, SocketAddr};
use thiserror::Error;

use tower_http::cors::{Any, CorsLayer};

/// Enum representing various errors that can occur during RPC operations.
#[derive(Error, Debug)]
pub enum RpcError {
    /// IO error.
    #[error(transparent)]
    IoError(#[from] std::io::Error),

    /// Error occurred during parsing.
    #[error(transparent)]
    ParseError(#[from] AddrParseError),

    /// Error related to JSON-RPC.
    #[error(transparent)]
    JsonRpcError(#[from] InvalidPath),

    /// Error related to Prometheus handler.
    #[error(transparent)]
    PrometheusHandlerError(#[from] crate::prometheus_handler::Error),

    /// Error related to Prometheus.
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

    // Creating the prometheus registry to register the metrics
    let registry = Registry::new();
    // register the metrics
    let metrics = RpcMetrics::new(Some(&registry))?.map(|m| MetricsLayer::new(m, "http"));
    tokio::spawn(async move {
        // serve the prometheus metrics on the given port so that it can be read
        let _ = init_prometheus(
            SocketAddr::new(
                std::net::IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                get_env_or_default("PROMETHEUS_PORT", "9615").parse().unwrap(),
            ),
            registry,
        )
        .await;
    });
    // add the metrics as a middleware to the RPC so that every new RPC call fires prometheus metrics
    // upon start, finish etc. we don't need to manually handle each method, it should automatically
    // work for any new method.
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
