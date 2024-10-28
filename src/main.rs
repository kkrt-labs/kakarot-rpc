use dotenvy::dotenv;
use eyre::Result;
use kakarot_rpc::{
    client::EthClient,
    constants::{KAKAROT_RPC_CONFIG, RPC_CONFIG},
    eth_rpc::{rpc::KakarotRpcModuleBuilder, run_server},
    pool::{
        constants::PRUNE_DURATION,
        mempool::{maintain_transaction_pool, AccountManager},
    },
    providers::eth_provider::database::Database,
};
use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
use opentelemetry_sdk::runtime::Tokio;
use reth_transaction_pool::PoolConfig;
use starknet::{
    core::types::Felt,
    providers::{jsonrpc::HttpTransport, JsonRpcClient},
};
use std::{env::var, str::FromStr, sync::Arc};
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    // Environment variables are safe to use after this
    dotenv().ok();

    setup_tracing().expect("failed to start tracing and metrics");

    let starknet_provider = JsonRpcClient::new(HttpTransport::new(KAKAROT_RPC_CONFIG.network_url.clone()));

    // Setup the database
    let db_client =
        mongodb::Client::with_uri_str(var("MONGO_CONNECTION_STRING").expect("Missing MONGO_CONNECTION_STRING .env"))
            .await?;
    let db = Database::new(
        db_client.database_with_options(
            &var("MONGO_DATABASE_NAME").expect("Missing MONGO_DATABASE_NAME from .env"),
            DatabaseOptions::builder()
                .read_concern(ReadConcern::majority())
                .write_concern(WriteConcern::majority())
                .build(),
        ),
    );

    // Setup the eth provider
    let starknet_provider = Arc::new(starknet_provider);

    // Get the pool config
    // TODO call Kakarot.get_base_fee
    let config = PoolConfig { minimal_protocol_basefee: 0, ..Default::default() };

    let eth_client = EthClient::new(starknet_provider, config, db.clone());
    let eth_client = Arc::new(eth_client);

    // Start the relayer manager
    let addresses =
        var("RELAYERS_ADDRESSES")?.split(',').filter_map(|addr| Felt::from_str(addr).ok()).collect::<Vec<_>>();
    AccountManager::from_addresses(addresses, Arc::clone(&eth_client)).await?.start();

    // Start the maintenance of the mempool
    maintain_transaction_pool(Arc::clone(&eth_client), PRUNE_DURATION);

    // Setup the RPC module
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(eth_client).rpc_module()?;

    // Start the RPC server
    let (socket_addr, server_handle) = run_server(kakarot_rpc_module, RPC_CONFIG.clone()).await?;
    let url = format!("http://{socket_addr}");

    tracing::info!("RPC Server running on {url}...");

    server_handle.stopped().await;

    Ok(())
}

/// Set up the subscriber for tracing and metrics
fn setup_tracing() -> Result<()> {
    // Prepare a tracer pipeline that exports to the OpenTelemetry collector,
    // using tonic as the gRPC client. Using a batch exporter for better performance:
    // https://docs.rs/opentelemetry-otlp/0.17.0/opentelemetry_otlp/#performance
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .install_batch(Tokio)?;
    // Set up the tracing layer with the OpenTelemetry tracer. A layer is a basic building block,
    // in tracing, that allows to define behavior for collecting or recording trace data. Layers
    // can be stacked on top of each other to create a pipeline.
    // https://docs.rs/tracing-subscriber/latest/tracing_subscriber/layer/trait.Layer.html
    let tracing_layer = tracing_opentelemetry::layer().with_tracer(tracer).boxed();

    // Prepare a metrics pipeline that exports to the OpenTelemetry collector.
    let metrics = opentelemetry_otlp::new_pipeline()
        .metrics(Tokio)
        .with_exporter(opentelemetry_otlp::new_exporter().tonic())
        .build()?;
    let metrics_layer = MetricsLayer::new(metrics).boxed();

    // Add a filter to the subscriber to control the verbosity of the logs
    let filter = var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let env_filter = EnvFilter::builder().parse(filter)?;

    // Stack the layers and initialize the subscriber
    let stacked_layer = tracing_layer.and_then(metrics_layer).and_then(env_filter);

    // Add a fmt subscriber
    let filter = EnvFilter::builder().from_env()?;
    let stdout = tracing_subscriber::fmt::layer().with_filter(filter).boxed();

    tracing_subscriber::registry().with(stacked_layer).with(stdout).init();

    Ok(())
}
