use crate::client::{constants::selectors::GET_STARKNET_CONTRACT_ADDRESS, StarknetClientImpl};
use jsonrpsee::server::ServerHandle;
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Debug)]
pub struct EthJsonRpcResponse<StarknetParams> {
    pub id: usize,
    pub jsonrpc: String,
    pub result: StarknetParams,
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

pub async fn setup_wiremock() -> String {
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
    // TODO: Change this method with associated correct code
    // let tx_calldata_vec =
    //     vec![FieldElement::from_hex_be("0xabde1007dcf45cb509ddde375162399a99880064").unwrap()];
    // let request = FunctionCall {
    //     contract_address: FieldElement::from_hex_be(ACCOUNT_REGISTRY_ADDRESS).unwrap(),
    //     entry_point_selector: GET_STARKNET_CONTRACT_ADDRESS,
    //     calldata: tx_calldata_vec,
    // };
    // let block_id = BlockId::Tag(BlockTag::Latest);
    // Mock::given(method("POST"))
    //     .and(body_json(StarknetRpcBaseData::call([
    //         serde_json::to_value(request).unwrap(),
    //         serde_json::to_value(block_id).unwrap(),
    //     ])))
    //     .respond_with(
    //         ResponseTemplate::new(200)
    //             .set_body_raw(
    //                 include_str!("data/starknet_getCode.json"),
    //                 "application/json",
    //             )
    //             .append_header("vary", "Accept-Encoding")
    //             .append_header("vary", "Origin"),
    //     )
    //     .mount(&mock_server)
    //     .await;

    // Get kakarot contract bytecode
    // TODO: Use the latest mapping between starknet and EVM adresses

    mock_server.uri()
}
