use std::net::SocketAddr;
use std::sync::Arc;

use jsonrpsee::server::ServerHandle;
use kakarot_rpc_core::client::api::KakarotStarknetApi;
use kakarot_rpc_core::client::config::{Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::test_utils::deploy_helpers::KakarotTestEnvironmentContext;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

use crate::config::RPCConfig;
use crate::rpc::KakarotRpcModuleBuilder;
use crate::run_server;

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
pub async fn start_kakarot_rpc_server(
    kakarot_test_env: &KakarotTestEnvironmentContext,
) -> Result<(SocketAddr, ServerHandle), eyre::Report> {
    let sequencer = kakarot_test_env.sequencer();
    let kakarot = kakarot_test_env.kakarot();

    let provider = Arc::new(JsonRpcClient::new(HttpTransport::new(sequencer.url())));
    let starknet_config = StarknetConfig::new(
        Network::JsonRpcProvider(sequencer.url()),
        kakarot.kakarot_address,
        kakarot.proxy_class_hash,
        kakarot.externally_owned_account_class_hash,
        kakarot.contract_account_class_hash,
    );

    let deployer_account = kakarot_test_env.client().deployer_account().clone();

    let kakarot_client = Arc::new(KakarotClient::new(starknet_config, provider, deployer_account));

    // Create and run Kakarot RPC module.
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()?;
    let rpc_config = RPCConfig::from_env()?;
    let (server_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    Ok((server_addr, server_handle))
}
