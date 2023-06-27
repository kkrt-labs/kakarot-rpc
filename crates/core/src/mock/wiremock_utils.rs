use std::str::FromStr;

use reqwest::StatusCode;
use reth_primitives::{BlockId, H256};
use serde::{Deserialize, Serialize};
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag, FieldElement};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use wiremock::matchers::{body_json, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::client::client_api::KakarotProvider;
use crate::client::helpers::ethers_block_id_to_starknet_block_id;
use crate::client::KakarotClient;

#[derive(Serialize, Debug)]
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
    pub const fn block_number(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_blockNumber", params }
    }

    pub const fn block_with_txs(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getBlockWithTxs", params }
    }

    pub const fn block_with_tx_hashes(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getBlockWithTxHashes", params }
    }

    pub const fn transaction_by_block_id_and_index(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getTransactionByBlockIdAndIndex", params }
    }

    pub const fn transaction_receipt(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getTransactionReceipt", params }
    }

    pub const fn transaction_by_hash(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getTransactionByHash", params }
    }

    pub const fn call(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_call", params }
    }

    pub const fn class_hash_at(params: StarknetParams) -> Self {
        Self { id: 1, jsonrpc: "2.0", method: "starknet_getClassHashAt", params }
    }
}

pub async fn setup_wiremock() -> String {
    let mock_server = MockServer::start().await;

    mock_block_number().mount(&mock_server).await;

    mock_block_with_txs().mount(&mock_server).await;

    mock_block_with_txs_hashes().mount(&mock_server).await;

    mock_block_with_txs_latest().mount(&mock_server).await;

    mock_block_with_txs_hashes_latest().mount(&mock_server).await;

    // block_with_txs & block_with_tx_hashes from pending
    mock_block_with_txs_pending().mount(&mock_server).await;

    mock_block_with_txs_hash_pending().mount(&mock_server).await;

    // transaction_by_block_hash_and_index from latest
    mock_transaction_by_block_hash_and_index_latest().mount(&mock_server).await;

    // * test_transaction_by_block_hash_and_index_is_ok
    // transaction_by_block_hash_and_index from block hash
    mock_transaction_by_block_hash_and_index().mount(&mock_server).await;

    // transaction_receipt for transaction_by_block_hash_and_index from block hash
    mock_transaction_receipt_for_transaction_by_block_hash_and_index().mount(&mock_server).await;

    // * test_transaction_receipt_invoke_is_ok
    mock_transaction_receipt_invoke().mount(&mock_server).await;

    mock_transaction_by_hash().mount(&mock_server).await;

    mock_get_code().mount(&mock_server).await;

    mock_get_evm_address().mount(&mock_server).await;

    mock_get_class_hash_at().mount(&mock_server).await;

    // Get kakarot contract bytecode
    // TODO: Use the latest mapping between starknet and EVM addresses

    mock_server.uri()
}

pub async fn setup_mock_client() -> Box<dyn KakarotProvider> {
    let starknet_rpc = setup_wiremock().await;
    Box::new(
        KakarotClient::new(
            &starknet_rpc,
            FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap(),
            FieldElement::from_hex_be("0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5").unwrap(),
        )
        .unwrap(),
    )
}

pub async fn setup_mock_client_crate() -> KakarotClient<JsonRpcClient<HttpTransport>>
where
    KakarotClient<JsonRpcClient<HttpTransport>>: KakarotProvider,
{
    let starknet_rpc = setup_wiremock().await;

    KakarotClient::new(
        &starknet_rpc,
        FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap(),
        FieldElement::from_hex_be("0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5").unwrap(),
    )
    .unwrap()
}

fn mock_block_number() -> Mock {
    Mock::given(method("POST")).and(body_json(StarknetRpcBaseData::block_number(Vec::<u8>::new()))).respond_with(
        response_template_with_status(StatusCode::OK)
            .set_body_raw(include_str!("data/blocks/starknet_blockNumber.json"), "application/json"),
    )
}

fn mock_block_with_txs() -> Mock {
    let block_id = BlockId::Hash(
        H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9").unwrap().into(),
    );
    let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id).unwrap();
    Mock::given(method("POST")).and(body_json(StarknetRpcBaseData::block_with_txs([&starknet_block_id]))).respond_with(
        response_template_with_status(StatusCode::OK)
            .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxs.json"), "application/json"),
    )
}

fn mock_block_with_txs_hashes() -> Mock {
    let block_id_tx_hashes = BlockId::Hash(
        H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e").unwrap().into(),
    );
    let starknet_block_id_tx_hashes = ethers_block_id_to_starknet_block_id(block_id_tx_hashes).unwrap();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_tx_hashes([&starknet_block_id_tx_hashes])))
        .respond_with(
            response_template_with_status(StatusCode::OK)
                .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxHashes.json"), "application/json"),
        )
}

fn mock_block_with_txs_latest() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST")).and(body_json(StarknetRpcBaseData::block_with_txs([&latest_block]))).respond_with(
        response_template_with_status(StatusCode::OK)
            .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxs.json"), "application/json"),
    )
}

fn mock_block_with_txs_hashes_latest() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST")).and(body_json(StarknetRpcBaseData::block_with_tx_hashes([&latest_block]))).respond_with(
        response_template_with_status(StatusCode::OK)
            .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxHashes.json"), "application/json"),
    )
}

fn mock_block_with_txs_pending() -> Mock {
    let pending_block = StarknetBlockId::Tag(BlockTag::Pending);
    Mock::given(method("POST")).and(body_json(StarknetRpcBaseData::block_with_txs([&pending_block]))).respond_with(
        response_template_with_status(StatusCode::OK)
            .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxs.json"), "application/json"),
    )
}

fn mock_block_with_txs_hash_pending() -> Mock {
    let pending_block = StarknetBlockId::Tag(BlockTag::Pending);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_tx_hashes([&pending_block])))
        .respond_with(
            response_template_with_status(StatusCode::OK)
                .set_body_raw(include_str!("data/blocks/starknet_getBlockWithTxHashes.json"), "application/json"),
        )
}

fn mock_transaction_by_block_hash_and_index_latest() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_by_block_id_and_index([
            serde_json::to_value(latest_block).unwrap(),
            serde_json::to_value(0).unwrap(),
        ])))
        .respond_with(response_template_with_status(StatusCode::OK).set_body_raw(
            include_str!("data/transactions/starknet_getTransactionByBlockIdAndIndex.json"),
            "application/json",
        ))
}

fn mock_transaction_by_block_hash_and_index() -> Mock {
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_by_block_id_and_index([
            serde_json::to_value(
                serde_json::json!({"block_hash":"0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9"}),
            )
            .unwrap(),
            serde_json::to_value(0).unwrap(),
        ])))
        .respond_with(response_template_with_status(StatusCode::OK).set_body_raw(
            include_str!("data/transactions/starknet_getTransactionByBlockIdAndIndex.json"),
            "application/json",
        ))
}

fn mock_transaction_receipt_for_transaction_by_block_hash_and_index() -> Mock {
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_receipt([
            "0x3204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c",
        ])))
        .respond_with(
            response_template_with_status(StatusCode::OK).set_body_raw(
                include_str!("data/transactions/starknet_getTransactionReceipt.json"),
                "application/json",
            ),
        )
}

fn mock_transaction_receipt_invoke() -> Mock {
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_receipt([
            "0x3ffcfea6eed902191033c88bded1e396a9aef4b88b32e6387eea30c83b84834",
        ])))
        .respond_with(response_template_with_status(StatusCode::OK).set_body_raw(
            include_str!("data/transactions/starknet_getTransactionReceipt_Invoke.json"),
            "application/json",
        ))
}

fn mock_transaction_by_hash() -> Mock {
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_by_hash([
            "0x3204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c",
        ])))
        .respond_with(response_template_with_status(StatusCode::OK).set_body_raw(
            include_str!("data/transactions/starknet_getTransactionByHash_Invoke.json"),
            "application/json",
        ))
}

fn mock_get_code() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    let get_code_call_request = serde_json::json!({
        "contract_address": "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
        "entry_point_selector": "0x2f22d9e1ae4a391b4a190b8225f2f6f772a083382b7ded3e8d85743a8fcfdcd",
        "calldata": [],
    });
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(get_code_call_request).unwrap(),
            serde_json::to_value(latest_block).unwrap(),
        ])))
        .respond_with(
            response_template_with_status(StatusCode::OK)
                .set_body_raw(include_str!("data/starknet_getCode.json"), "application/json"),
        )
}

fn mock_get_evm_address() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    let get_evm_address_call_request = serde_json::json!({
        "contract_address": "0x744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485",
        "entry_point_selector": "0x158359fe4236681f6236a2f303f9350495f73f078c9afd1ca0890fa4143c2ed",
        "calldata": [],
    });
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(get_evm_address_call_request).unwrap(),
            serde_json::to_value(latest_block).unwrap(),
        ])))
        .respond_with(
            response_template_with_status(StatusCode::OK)
                .set_body_raw(include_str!("data/kakarot_getEvmAddress.json"), "application/json"),
        )
}

fn mock_get_class_hash_at() -> Mock {
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::class_hash_at([
            serde_json::to_value(latest_block).unwrap(),
            serde_json::to_value("0x744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485").unwrap(),
        ])))
        .respond_with(
            response_template_with_status(StatusCode::OK)
                .set_body_raw(include_str!("data/transactions/starknet_getClassHashAt.json"), "application/json"),
        )
}

fn response_template_with_status(status_code: StatusCode) -> ResponseTemplate {
    ResponseTemplate::new(status_code).append_header("vary", "Accept-Encoding").append_header("vary", "Origin")
}
