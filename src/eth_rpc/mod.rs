use std::convert::Infallible;
// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, Ipv4Addr, SocketAddr};
use std::str::FromStr;

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
use hyper::{
    server::conn::AddrStream,
    service::{make_service_fn, service_fn},
};
use jsonrpsee::server::middleware::http::{InvalidPath, ProxyGetRequestLayer};
use jsonrpsee::server::{
    stop_channel, RpcServiceBuilder, ServerBuilder, ServerHandle, StopHandle, TowerServiceBuilder,
};
use jsonrpsee::RpcModule;
use prometheus::Registry;
use thiserror::Error;
use tokio::net::TcpListener;
use tower::Service;
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
    HyperError(#[from] hyper::Error),
    #[error(transparent)]
    PrometheusHandlerError(#[from] crate::prometheus_handler::Error),
    #[error(transparent)]
    PrometheusError(#[from] prometheus::Error),
}

#[derive(Debug, Clone)]
struct PerConnection<RpcMiddleware, HttpMiddleware> {
    stop_handle: StopHandle,
    metrics: Option<RpcMetrics>,
    service_builder: TowerServiceBuilder<RpcMiddleware, HttpMiddleware>,
    rpc_module: RpcModule<()>,
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

    let builder = ServerBuilder::default()
        .max_connections(get_env_or_default("RPC_MAX_CONNECTIONS", "100").parse().unwrap())
        .set_http_middleware(http_middleware)
        .to_service_builder();
    let (stop_handle, server_handle) = stop_channel();

    let registry = Registry::new();
    let cfg = PerConnection {
        stop_handle: stop_handle.clone(),
        metrics: RpcMetrics::new(Some(&registry))?,
        service_builder: builder.clone(),
        rpc_module: kakarot_rpc_module,
    };
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

    let make_service = make_service_fn(move |_conn: &AddrStream| {
        let cfg = cfg.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let PerConnection { service_builder, metrics, stop_handle, rpc_module } = cfg.clone();
                let metrics = metrics.map(|m| MetricsLayer::new(m, "http"));

                let rpc_middleware = RpcServiceBuilder::new().option_layer(metrics);

                let mut svc =
                    service_builder.set_rpc_middleware(rpc_middleware).build(build_rpc_api(rpc_module), stop_handle);

                async move { svc.call(req).await }
            }))
        }
    });

    let addr = SocketAddr::from_str(socket_addr.as_str())?;
    let std_listener = TcpListener::bind(addr).await?.into_std()?;
    let server = hyper::Server::from_tcp(std_listener)?.serve(make_service);

    tokio::spawn(async move {
        let graceful = server.with_graceful_shutdown(async move { stop_handle.shutdown().await });
        let _ = graceful.await;
    });

    Ok((addr, server_handle))
}

fn get_env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn build_rpc_api<M: Send + Sync + 'static>(mut rpc_api: RpcModule<M>) -> RpcModule<M> {
    let mut available_methods = rpc_api.method_names().collect::<Vec<_>>();
    // The "rpc_methods" is defined below and we want it to be part of the reported methods.
    available_methods.push("rpc_methods");
    available_methods.sort();

    rpc_api
        .register_method("rpc_methods", move |_, _| {
            serde_json::json!({
                "methods": available_methods,
            })
        })
        .expect("infallible all other methods have their own address space; qed");

    rpc_api
}
