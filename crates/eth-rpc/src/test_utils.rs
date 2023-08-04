use std::net::SocketAddr;
use std::sync::Arc;

use dojo_test_utils::sequencer::TestSequencer;
use jsonrpsee::server::ServerHandle;
use kakarot_rpc_core::client::config::{Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
use kakarot_rpc_core::test_utils::deploy_helpers::deploy_kakarot_system;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
use starknet::providers::JsonRpcClient as StarknetJsonRpcClient;

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
/// use kakarot_rpc::test_utils::setup_kakarot_rpc_integration_env;
/// use dojo_test_utils::sequencer::TestSequencer;
/// use std::sync::Arc;
/// use tokio::runtime::Runtime;
///
/// #[tokio::test]
/// async fn test_case() {
///    // Create a TestSequencer.
///    let test_sequencer = Arc::new(TestSequencer::new());
///
///    // Set up the Kakarot RPC integration environment.
///    let (server_addr, server_handle) = setup_kakarot_rpc_integration_env(&test_sequencer).await.unwrap();
///    
///    // Query whatever eth_rpc endpoints
///     
///    // Dont forget to close server at the end.
///    server_handle.stop().expect("Failed to stop the server");
///
/// }
/// ```
pub async fn setup_kakarot_rpc_integration_env(
    starknet_test_sequencer: &Arc<TestSequencer>,
) -> Result<(SocketAddr, ServerHandle), eyre::Report> {
    // Define the funding amount.
    let funding_amount = FieldElement::from_dec_str("1000000000000000000")?;

    // Deploy Kakarot contracts.
    let deployed_kakarot = deploy_kakarot_system(starknet_test_sequencer, EOA_WALLET.clone(), funding_amount).await;

    // Initialize StarknetHttpTransport with sequencer's URL.
    let starknet_http_transport = StarknetHttpTransport::new(starknet_test_sequencer.url());

    // Create Starknet and Kakarot clients.
    let starknet_config = StarknetConfig::new(
        Network::JsonRpcProvider(starknet_test_sequencer.url()),
        deployed_kakarot.kakarot_address,
        deployed_kakarot.proxy_class_hash,
    );
    let starknet_client = StarknetJsonRpcClient::new(starknet_http_transport);
    let kakarot_client = Arc::new(KakarotClient::new(starknet_config, starknet_client));

    // Create and run Kakarot RPC module.
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()?;
    let rpc_config = RPCConfig::from_env()?;
    let (server_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    Ok((server_addr, server_handle))
}
