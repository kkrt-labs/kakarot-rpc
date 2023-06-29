use kakarot_rpc::eth_rpc::KakarotEthRpc;
use kakarot_rpc_core::client::client_api::KakarotProvider;
use kakarot_rpc_core::client::config::StarknetConfig;
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::mock::wiremock_utils::setup_wiremock;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcTransport};
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
pub async fn setup_kakarot_eth_rpc<T: JsonRpcTransport + Send + Sync>() -> KakarotEthRpc<T>
where
    KakarotClient<JsonRpcClient<HttpTransport>>: KakarotProvider<T>,
{
    let starknet_rpc = setup_wiremock().await;
    let kakarot_address =
        FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap();
    let proxy_account_class_hash =
        FieldElement::from_hex_be("0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5").unwrap();

    let kakarot_client =
        KakarotClient::new(StarknetConfig::new(&starknet_rpc, kakarot_address, proxy_account_class_hash)).unwrap();

    KakarotEthRpc::new(Box::new(kakarot_client))
}
