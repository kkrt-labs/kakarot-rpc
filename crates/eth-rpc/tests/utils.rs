use std::net::SocketAddr;
use std::sync::Arc;

use dojo_test_utils::rpc::MockJsonRpcTransport;
use dojo_test_utils::sequencer::TestSequencer;
use jsonrpsee::server::ServerHandle;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::rpc::KakarotRpcModuleBuilder;
use kakarot_rpc::run_server;
use kakarot_rpc::servers::eth_rpc::KakarotEthRpc;
use kakarot_rpc_core::client::config::{Network, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::mock::mock_starknet::{all_fixtures, init_mock_client};
use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::HttpTransport as StarknetHttpTransport;
use starknet::providers::{JsonRpcClient, JsonRpcClient as StarknetJsonRpcClient};

/// Run wiremock to fake Starknet rpc and then run our own `kakarot_rpc_server`.
///
/// Example :
/// ```ignore
///   use kakarot_rpc::test_utils::setup_rpc_server;
///
///   #[tokio::test]
///   async fn test_case() {
///       // Run base server
///       let (_, server_handle) = setup_rpc_server().await;
///
///       // Query whatever eth_rpc endpoints
///       let client = reqwest::Client::new();
///        let res = client
///            .post("http://127.0.0.1:3030")
///            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_chainId\", \"params\": [] }")
///            .header("content-type", "application/json")
///            .send()
///            .await
///            .unwrap();
///
///        // Dont forget to close server at the end.
///        let _has_stop = server_handle.stop().unwrap();
///   }
/// ```
pub async fn setup_mock_eth_rpc() -> KakarotEthRpc<JsonRpcClient<MockJsonRpcTransport>> {
    let kakarot_client = init_mock_client(Some(all_fixtures()));

    KakarotEthRpc::new(Arc::new(kakarot_client))
}

/// Deploys the Kakarot contracts, starts the Kakarot RPC server, and returns the server's address
/// and a handle to stop the server.
///
/// This function:
/// 1. Creates a Starknet test sequencer
/// 2. Deploys the Kakarot contracts
/// 3. Starts the Kakarot RPC server
///
/// # Arguments
///
/// * No arguments.
///
/// # Returns
///
/// A Result with a tuple on successful execution. The tuple contains the server's address
/// (SocketAddr), a handle (ServerHandle) to stop the server, and the test sequencer
/// (Arc<TestSequencer>).
///
/// # Errors
///
/// Returns an Err variant of eyre::Report if:
/// * There's an issue deploying the Kakarot contracts.
/// * There's an issue constructing the Starknet test sequencer.
/// * There's an issue running the RPC server.
pub async fn setup_kakarot_rpc_integration_env() -> Result<(SocketAddr, ServerHandle, Arc<TestSequencer>), eyre::Report>
{
    // Define expected funded amount.
    let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000")?;

    // Create Starknet test sequencer and deploy Kakarot contracts.
    let starknet_test_sequencer = Arc::new(construct_kakarot_test_sequencer().await);
    let deploying_kakarot = deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount);

    // Initialize StarknetHttpTransport with sequencer's URL.
    let starknet_http_transport = StarknetHttpTransport::new(starknet_test_sequencer.url());

    // Await for the deploy_kakarot_system future and proceed with the initialization.
    let deployed_kakarot = deploying_kakarot.await;

    // Create Starknet and Kakarot clients.
    let starknet_config = StarknetConfig::new(
        Network::JsonRpcProvider(starknet_test_sequencer.url()),
        deployed_kakarot.kakarot,
        deployed_kakarot.kakarot_proxy,
    );
    let starknet_client = StarknetJsonRpcClient::new(starknet_http_transport);
    let kakarot_client = Arc::new(KakarotClient::new(starknet_config, starknet_client));

    // Create and run Kakarot RPC module.
    let kakarot_rpc_module = KakarotRpcModuleBuilder::new(kakarot_client).rpc_module()?;
    let rpc_config = RPCConfig::from_env()?;
    let (server_addr, server_handle) = run_server(kakarot_rpc_module, rpc_config).await?;

    Ok((server_addr, server_handle, starknet_test_sequencer))
}
