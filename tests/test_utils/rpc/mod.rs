use std::net::SocketAddr;
use std::sync::Arc;

use jsonrpsee::server::ServerHandle;
use kakarot_rpc::eth_rpc::config::RPCConfig;
use kakarot_rpc::eth_rpc::rpc::KakarotRpcModuleBuilder;
use kakarot_rpc::eth_rpc::run_server;
use kakarot_rpc::starknet_client::config::{KakarotRpcConfig, Network};
use kakarot_rpc::starknet_client::KakarotClient;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

use super::sequencer::Katana;

/// Sets up the environment for Kakarot RPC integration tests by deploying the Kakarot contracts
/// and starting the Kakarot RPC server.
///
/// This function:
/// 1. Takes an `Arc<TestSequencer>` as input, which is used to deploy the Kakarot contracts and to
///    set up the Kakarot RPC server.
/// 2. Deploys the Kakarot contracts.
/// 3. Creates Starknet and Kakarot clients.
/// 4. Sets up and runs the Kakarot RPC module.
///
/// # Arguments
///
/// * `starknet_test_sequencer` - An Arc-wrapped TestSequencer. This is used to deploy the Kakarot
///   contracts and to set up the Kakarot RPC server.
///
/// # Returns
///
/// This function returns a Result containing a tuple with the server's address and a handle to
/// stop the server upon successful execution.
///
/// The function may return an Err variant of eyre::Report if there are issues with deploying the
/// Kakarot contracts, creating the clients, or running the RPC server.
///
/// # Example
/// ```ignore
/// use kakarot_rpc::test_utils::start_kakarot_rpc_server;
/// use kakarot_rpc_core::test_utils::fixtures::kakarot_test_env_ctx;
/// use kakarot_rpc_core::test_utils::deploy_helpers::KakarotTestEnvironmentContext;
/// use dojo_test_utils::sequencer::TestSequencer;
/// use std::sync::Arc;
/// use tokio::runtime::Runtime;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_case(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
///    // Set up the Kakarot RPC integration environment.
///    let (server_addr, server_handle) = start_kakarot_rpc_server(&kakarot_test_env_ctx).await.unwrap();
///
///    // Query whatever eth_rpc endpoints
///
///    // Dont forget to close server at the end.
///    server_handle.stop().expect("Failed to stop the server");
///
/// }
/// ```
///
/// allow(dead_code) is used because this function is used in tests,
/// and each test is compiled separately, so the compiler thinks this function is unused
#[allow(dead_code)]
pub async fn start_kakarot_rpc_server(katana: &Katana) -> Result<(SocketAddr, ServerHandle), eyre::Report> {
    let sequencer = katana.sequencer();
    let client = katana.client();

    let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(sequencer.url())));
    let starknet_config = KakarotRpcConfig::new(
        Network::JsonRpcProvider(sequencer.url()),
        client.kakarot_address(),
        client.proxy_account_class_hash(),
        client.externally_owned_account_class_hash(),
        client.contract_account_class_hash(),
    );

    let kakarot_client = Arc::new(KakarotClient::new(starknet_config, provider));

    // Create and run Kakarot RPC module.
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()?;
    let rpc_config = RPCConfig::from_env()?;
    let (server_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    Ok((server_addr, server_handle))
}
