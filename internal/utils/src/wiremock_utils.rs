use jsonrpsee::server::ServerHandle;
use kakarot_rpc::run_server;
use kakarot_rpc_core::lightclient::{
    constants::{selectors::GET_STARKNET_CONTRACT_ADDRESS, ACCOUNT_REGISTRY_ADDRESS},
    StarknetClientImpl,
};
use serde::Serialize;
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::models::{BlockId, BlockTag, FunctionCall},
};
use wiremock::{
    matchers::{body_json, method},
    Mock, MockServer, ResponseTemplate,
};

#[derive(Serialize)]
pub struct StarknetRpcBaseData<'a, StarknetParams> {
    id: usize,
    jsonrpc: &'a str,
    method: &'a str,
    params: StarknetParams,
}

#[derive(Serialize)]
pub struct GetBlockWithTx {
    pub block_id: BlockId,
}

#[derive(Serialize)]
pub struct EthGetChainId {
    pub block_id: BlockId,
}

impl<'a, StarknetParams> StarknetRpcBaseData<'a, StarknetParams> {
    pub fn get_block_number(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_blockNumber",
            params,
        }
    }

    pub fn get_block_with_tx(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getBlockWithTxs",
            params,
        }
    }

    pub fn get_block_with_tx_hashes(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getBlockWithTxHashes",
            params,
        }
    }

    pub fn call(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_call",
            params,
        }
    }
}

async fn setup_wiremock() -> String {
    let mock_server = MockServer::start().await;
    let empty_vec: Vec<&str> = Vec::new();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_number(&empty_vec)))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/starknet_blockNumber.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_tx([
            BlockId::Tag(BlockTag::Latest),
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/starknet_getBlockWithTxs.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_tx_hashes([
            BlockId::Tag(BlockTag::Latest),
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/starknet_getBlockWithTxHashes.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // Get kakarot contract addess
    let tx_calldata_vec =
        vec![FieldElement::from_hex_be("0xabde1007dcf45cb509ddde375162399a99880064").unwrap()];
    let request = FunctionCall {
        contract_address: FieldElement::from_hex_be(ACCOUNT_REGISTRY_ADDRESS).unwrap(),
        entry_point_selector: GET_STARKNET_CONTRACT_ADDRESS,
        calldata: tx_calldata_vec,
    };
    let block_id = BlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(request).unwrap(),
            serde_json::to_value(block_id).unwrap(),
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/starknet_getCode.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // Get kakarot contract bytecode
    // todo!();

    mock_server.uri()
}

/// Run wiremock to fake starknet rpc and then run our own kakarot_rpc_server.
///
/// Example :
/// ```
///   use kakarot_rpc_utils::wiremock_utils::setup_rpc_server;
///
///   #[tokio::test]
///   async fn test_case() {
///       // Run base server
///       let (_, server_handle) = setup_rpc_server().await;
///
///       //Query whatever eth_rpc endpoints
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
pub async fn setup_rpc_server() -> (String, ServerHandle) {
    let starknet_rpc = setup_wiremock().await;

    let starknet_lightclient = StarknetClientImpl::new(&starknet_rpc).unwrap();
    let (_rpc_server_uri, server_handle) =
        run_server(Box::new(starknet_lightclient)).await.unwrap();
    (starknet_rpc, server_handle)
}
