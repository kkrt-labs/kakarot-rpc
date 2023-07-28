use std::sync::Arc;

use dojo_test_utils::rpc::MockJsonRpcTransport;
use kakarot_rpc::servers::eth_rpc::KakarotEthRpc;
use kakarot_rpc_core::mock::mock_starknet::{all_fixtures, init_mock_client};
use starknet::providers::JsonRpcClient;

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
pub async fn setup_mock_eth_rpc() -> KakarotEthRpc<JsonRpcClient<MockJsonRpcTransport>> {
    let kakarot_client = init_mock_client(Some(all_fixtures()));

    KakarotEthRpc::new(Arc::new(kakarot_client))
}
