use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use dojo_test_utils::sequencer::TestSequencer;
use jsonrpsee::server::ServerHandle;
use kakarot_rpc::config::RPCConfig;
use kakarot_rpc::eth_rpc::KakarotEthRpc;
use kakarot_rpc::run_server;
use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, StarknetConfig};
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::mock::wiremock_utils::setup_wiremock;
use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
use kakarot_rpc_core::test_utils::deploy_helpers::{construct_kakarot_test_sequencer, deploy_kakarot_system};
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, HttpTransport as StarknetHttpTransport};
use starknet::providers::{JsonRpcClient, JsonRpcClient as StarknetJsonRpcClient};

/// Run wiremock to fake starknet rpc and then run our own `kakarot_rpc_server`.
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
pub async fn setup_kakarot_eth_rpc() -> KakarotEthRpc<JsonRpcClient<HttpTransport>> {
    let starknet_rpc = setup_wiremock().await;
    let kakarot_address =
        FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap();
    let proxy_account_class_hash =
        FieldElement::from_hex_be("0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5").unwrap();

    let config = StarknetConfig::new(starknet_rpc, kakarot_address, proxy_account_class_hash);
    let provider = JsonRpcClientBuilder::with_http(&config).unwrap().build();

    let kakarot_client = KakarotClient::new(config, provider);

    KakarotEthRpc::new(Box::new(kakarot_client))
}

pub async fn setup_kakarot_integration(starknet_test_sequencer: &Arc<TestSequencer>) -> (SocketAddr, ServerHandle) {
    // SUMMON THE STARKNET TEST SEQUENCER

    let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

    // UNLEASH THE MIGHTY KAKAROT
    let deployed_kakarot =
        deploy_kakarot_system(starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

    // INITIATE THE ALMIGHTY KAKAROT CLIENT
    let kakarot_client = KakarotClient::new(
        StarknetConfig::new(
            starknet_test_sequencer.url().as_ref().to_string(),
            deployed_kakarot.kakarot,
            deployed_kakarot.kakarot_proxy,
        ),
        StarknetJsonRpcClient::new(StarknetHttpTransport::new(starknet_test_sequencer.url())),
    );

    let rpc_config = RPCConfig::from_env().expect("config not found");

    // AND NOW THE RPC SERVER
    let (server_addr, server_handle) = run_server(Box::new(kakarot_client), rpc_config).await.unwrap();

    (server_addr, server_handle)
}

/// Asynchronously runs a Kakarot test using the provided client code.
///
/// This function creates a Starknet test sequencer, sets up a Kakarot RPC server, and then runs the
/// client code with the server's port number as an argument. It then spawns two tasks: one to run
/// the server until it's stopped, and one to run the client code. The server and client code are
/// run concurrently.
///
/// The function panics if the server task finishes before the client task. If the client task
/// returns an error, that error is propagated as a panic.
///
/// # Arguments
///
/// * `client_code` - A closure which takes a port number (u16) and returns a `Future` that yields
///   `()`. This is the code to be run as the client for the test. The future must be `'static` and
///   `Send` to allow it to be run across threads.
///
/// # Type Parameters
///
/// * `F`: The type of the closure `client_code`.
/// * `Fut`: The type of the future returned by `client_code`.
///
/// # Panics
///
/// Panics if the server task finishes before the client task, or if the client task returns an
/// error.
///
/// # Returns
///
/// This function is `async` and does not return a value. It should be `await`ed.
pub async fn run_kakarot_integration<F, Fut>(client_code: F)
where
    F: FnOnce(u16) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    let starknet_test_sequencer = Arc::new(construct_kakarot_test_sequencer().await);

    let (server_addr, server_handle) = setup_kakarot_integration(&starknet_test_sequencer).await;

    let client_code = client_code(server_addr.port());

    // without this extra touch, the starknet provider thread willl complete before
    // our client task can make its necessary interaction with the kakarot rpc
    let starknet_test_sequencer_clone = Arc::clone(&starknet_test_sequencer);
    let server_task = tokio::spawn(async move {
        let _ = starknet_test_sequencer_clone;
        server_handle.stopped().await
    });

    let client_task = tokio::spawn(client_code);

    // Wait for both the server and client to finish.
    tokio::select! {
        _ = server_task => {
            panic!("Server task finished first (this shouldn't happen).");
        }
        result = client_task => {

            if let Err(e) = result {
                std::panic::resume_unwind(e.into_panic())
            } else {
                println!("Client task finished, proceeding to shut down server.");
            }

        }
    };
}
