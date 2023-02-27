use std::str::FromStr;

use crate::helpers::ethers_block_id_to_starknet_block_id;
use reth_primitives::{BlockId, H256};
use serde::{Deserialize, Serialize};
use starknet::providers::jsonrpc::models::{BlockId as StarknetBlockId, BlockTag};
use wiremock::{
    matchers::{body_json, method},
    Mock, MockServer, ResponseTemplate,
};

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
    pub fn block_number(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_blockNumber",
            params,
        }
    }

    pub fn get_block_transaction_count(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getBlockTransactionCount",
            params,
        }
    }

    pub fn block_with_txs(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getBlockWithTxs",
            params,
        }
    }

    pub fn block_with_tx_hashes(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getBlockWithTxHashes",
            params,
        }
    }

    pub fn transaction_by_block_id_and_index(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getTransactionByBlockIdAndIndex",
            params,
        }
    }

    pub fn transaction_receipt(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getTransactionReceipt",
            params,
        }
    }

    pub fn transaction_by_hash(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getTransactionByHash",
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

    pub fn class_hash_at(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getClassHashAt",
            params,
        }
    }
}

pub async fn setup_wiremock() -> String {
    let mock_server = MockServer::start().await;

    // block_number
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_number(())))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_blockNumber.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // block_with_txs
    let block_id = BlockId::Hash(
        H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
            .unwrap()
            .into(),
    );
    let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id).unwrap();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_txs([
            &starknet_block_id,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxs.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // block_with_tx_hashes
    let block_id_tx_hashes = BlockId::Hash(
        H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e")
            .unwrap()
            .into(),
    );
    let starknet_block_id_tx_hashes =
        ethers_block_id_to_starknet_block_id(block_id_tx_hashes).unwrap();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_tx_hashes([
            &starknet_block_id_tx_hashes,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxHashes.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // block_with_txs latest
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_txs([
            &latest_block,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxs.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // block_with_tx_hashes latest
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_tx_hashes([
            &latest_block,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxHashes.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // block_with_txs & block_with_tx_hashes from pending
    let pending_block = StarknetBlockId::Tag(BlockTag::Pending);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_txs([
            &pending_block,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxs.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::block_with_tx_hashes([
            &pending_block,
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/blocks/starknet_getBlockWithTxHashes.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // transaction_by_block_hash_and_index from latest
    Mock::given(method("POST"))
        .and(body_json(
            StarknetRpcBaseData::transaction_by_block_id_and_index([
                serde_json::to_value(&latest_block).unwrap(),
                serde_json::to_value(0).unwrap(),
            ]),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getTransactionByBlockIdAndIndex.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // * test_transaction_by_block_hash_and_index_is_ok
    // transaction_by_block_hash_and_index from block hash
    Mock::given(method("POST"))
        .and(body_json(
            StarknetRpcBaseData::transaction_by_block_id_and_index([
                serde_json::to_value(serde_json::json!({"block_hash":"0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9"})).unwrap(),
                serde_json::to_value(0).unwrap(),
            ]),
        ))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getTransactionByBlockIdAndIndex.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // transaction_receipt for transaction_by_block_hash_and_index from block hash
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_receipt([
            "0x7c5df940744056d337c3de6e8f4500db4b9bfc821eb534b891555e90c39c048",
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getTransactionReceipt.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // * test_transaction_receipt_invoke_is_ok
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_receipt([
            "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getTransactionReceipt_Invoke.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::transaction_by_hash([
            "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getTransactionByHash_Invoke.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    let get_code_call_request = serde_json::json!({
        "contract_address": "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
        "entry_point_selector": "0x2f22d9e1ae4a391b4a190b8225f2f6f772a083382b7ded3e8d85743a8fcfdcd",
        "calldata": [],
    });
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(get_code_call_request).unwrap(),
            serde_json::to_value(&latest_block).unwrap(),
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

    let get_evm_address_call_request = serde_json::json!({
        "contract_address": "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
        "entry_point_selector": "0x158359fe4236681f6236a2f303f9350495f73f078c9afd1ca0890fa4143c2ed",
        "calldata": [],
    });
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(get_evm_address_call_request).unwrap(),
            serde_json::to_value(&latest_block).unwrap(),
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/kakarot_getEvmAddress.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::class_hash_at([
            serde_json::to_value(&latest_block).unwrap(),
            serde_json::to_value(
                "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
            )
            .unwrap(),
        ])))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(
                    include_str!("data/transactions/starknet_getClassHashAt.json"),
                    "application/json",
                )
                .append_header("vary", "Accept-Encoding")
                .append_header("vary", "Origin"),
        )
        .mount(&mock_server)
        .await;

    // Get kakarot contract bytecode
    // TODO: Use the latest mapping between starknet and EVM adresses

    mock_server.uri()
}
