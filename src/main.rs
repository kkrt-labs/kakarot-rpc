use dotenvy::dotenv;
use eyre::Result;
use kakarot_rpc::{
    client::EthClient,
    config::KakarotRpcConfig,
    eth_rpc::{config::RPCConfig, rpc::KakarotRpcModuleBuilder, run_server},
    pool::RetryHandler,
    providers::eth_provider::database::Database,
};
use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
use opentelemetry_sdk::runtime::Tokio;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use std::{env::var, sync::Arc};
use tracing_opentelemetry::MetricsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

#[tokio::main]
async fn main() -> Result<()> {
    // Environment variables are safe to use after this
    dotenv().ok();

    setup_tracing().expect("failed to start tracing and metrics");

    // Load the configuration
    let starknet_config = KakarotRpcConfig::from_env()?;
    let rpc_config = RPCConfig::from_env()?;

    let starknet_provider = JsonRpcClient::new(HttpTransport::new(starknet_config.network_url));

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

    // Setup hive
    #[cfg(feature = "hive")]
    setup_hive(&starknet_provider).await?;

    // Setup the eth provider
    let starknet_provider = Arc::new(starknet_provider);

    let eth_client = EthClient::try_new(starknet_provider, db.clone()).await.expect("failed to start ethereum client");
    let eth_provider = eth_client.eth_provider().clone();

    // Setup the retry handler
    let retry_handler = RetryHandler::new(eth_provider, db);
    retry_handler.start(&tokio::runtime::Handle::current());

    // Setup the RPC module
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(eth_client).rpc_module()?;

    // Start the RPC server
    let (socket_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

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
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let env_filter = EnvFilter::builder().parse(filter)?;

    // Stack the layers and initialize the subscriber
    let stacked_layer = tracing_layer.and_then(metrics_layer).and_then(env_filter);
    tracing_subscriber::registry().with(stacked_layer).init();

    Ok(())
}

#[allow(clippy::significant_drop_tightening)]
#[cfg(feature = "hive")]
async fn setup_hive(starknet_provider: &JsonRpcClient<HttpTransport>) -> Result<()> {
    use kakarot_rpc::providers::eth_provider::constant::hive::{CHAIN_ID, DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
    use starknet::{accounts::ConnectedAccount, core::types::Felt, providers::Provider as _};

    let chain_id = starknet_provider.chain_id().await?;
    let chain_id: u64 = (Felt::from(u64::MAX).to_bigint() & chain_id.to_bigint()).try_into()?;

    CHAIN_ID.set(chain_id.into()).expect("Failed to set chain id");

    let deployer_nonce = DEPLOY_WALLET.get_nonce().await?;
    let mut nonce = DEPLOY_WALLET_NONCE.lock().await;
    *nonce = deployer_nonce;

    Ok(())
}
