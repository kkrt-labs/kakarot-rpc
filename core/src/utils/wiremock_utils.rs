use std::str::FromStr;

use reth_primitives::rpc::{BlockId, H256};
use serde::{de::Error as DeError, Deserialize, Deserializer, Serialize, Serializer};
use serde_with::{serde_as, DeserializeAs, SerializeAs};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId as StarknetBlockId, BlockTag},
        JsonRpcMethod,
    },
};
use wiremock::{
    matchers::{body_json, method},
    Mock, MockServer, ResponseTemplate,
};

use crate::helpers::ethers_block_id_to_starknet_block_id;

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
    pub fn get_block_number(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_blockNumber",
            params,
        }
    }

    pub fn get_block_with_txs(params: StarknetParams) -> Self {
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

    pub fn get_transaction_by_block_id_and_index(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getTransactionByBlockIdAndIndex",
            params,
        }
    }

    pub fn get_transaction_receipt(params: StarknetParams) -> Self {
        Self {
            id: 1,
            jsonrpc: "2.0",
            method: "starknet_getTransactionReceipt",
            params,
        }
    }

    pub fn get_transaction_by_hash(params: StarknetParams) -> Self {
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
}

// DEBUG FUNCTION
pub struct UfeHex;

impl SerializeAs<FieldElement> for UfeHex {
    fn serialize_as<S>(value: &FieldElement, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{value:#x}"))
    }
}

impl<'de> DeserializeAs<'de, FieldElement> for UfeHex {
    fn deserialize_as<D>(deserializer: D) -> Result<FieldElement, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match FieldElement::from_hex_be(&value) {
            Ok(value) => Ok(value),
            Err(err) => Err(DeError::custom(format!("invalid hex string: {err}"))),
        }
    }
}

#[serde_as]
#[derive(Serialize, Deserialize)]
struct Felt(#[serde_as(as = "UfeHex")] pub FieldElement);

#[derive(Debug, Serialize)]
struct JsonRpcRequest<T> {
    id: u64,
    jsonrpc: &'static str,
    method: JsonRpcMethod,
    params: T,
}

pub fn debug_entry_param(method: JsonRpcMethod, params: Vec<serde_json::Value>) -> String {
    let request = JsonRpcRequest {
        id: 1,
        jsonrpc: "2.0",
        method,
        params,
    };
    println!(
        "JsonRpcRequest: {:?}",
        serde_json::to_string(&request).unwrap()
    );
    serde_json::to_string(&request).unwrap()
}

// END DEBUG FUNCTION

pub async fn setup_wiremock() -> String {
    let mock_server = MockServer::start().await;

    // get_block_number
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_number(())))
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

    // get_block_with_txs
    let block_id = BlockId::Hash(
        H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
            .unwrap(),
    );
    let starknet_block_id = ethers_block_id_to_starknet_block_id(block_id).unwrap();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_txs([
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

    // get_block_with_tx_hashes
    let block_id_tx_hashes = BlockId::Hash(
        H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e")
            .unwrap(),
    );
    let starknet_block_id_tx_hashes =
        ethers_block_id_to_starknet_block_id(block_id_tx_hashes).unwrap();
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_tx_hashes([
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

    // get_block_with_txs latest
    let latest_block = StarknetBlockId::Tag(BlockTag::Latest);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_txs([
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

    // get_block_with_tx_hashes latest
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_tx_hashes([
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

    // get_block_with_txs & get_block_with_tx_hashes from pending
    let pending_block = StarknetBlockId::Tag(BlockTag::Pending);
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_block_with_txs([
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
        .and(body_json(StarknetRpcBaseData::get_block_with_tx_hashes([
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
            StarknetRpcBaseData::get_transaction_by_block_id_and_index([
                serde_json::to_value(&latest_block).unwrap(),
                serde_json::to_value(1).unwrap(),
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
            StarknetRpcBaseData::get_transaction_by_block_id_and_index([
                serde_json::to_value(serde_json::json!({"block_hash":"0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9"})).unwrap(),
                serde_json::to_value(1).unwrap(),
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

    // get_transaction_receipt for transaction_by_block_hash_and_index from block hash
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::get_transaction_receipt([
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
        .and(body_json(StarknetRpcBaseData::get_transaction_receipt([
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
        .and(body_json(StarknetRpcBaseData::get_transaction_by_hash([
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

    // Get kakarot contract addess
    // let tx_calldata_vec =
    //     vec![FieldElement::from_hex_be("0xabde1007dcf45cb509ddde375162399a99880064").unwrap()];
    // let request = FunctionCall {
    //     contract_address: FieldElement::from_hex_be(ACCOUNT_REGISTRY_ADDRESS).unwrap(),
    //     entry_point_selector: GET_STARKNET_CONTRACT_ADDRESS,
    //     calldata: tx_calldata_vec,
    // };

    let call_request = serde_json::json!({
        "contract_address": "0x46bfa580e4fa55a38eaa7f51a3469f86b336eed59a6136a07b7adcd095b0eb2",
        "entry_point_selector": "0x158359fe4236681f6236a2f303f9350495f73f078c9afd1ca0890fa4143c2ed",
        "calldata": [],
    });
    Mock::given(method("POST"))
        .and(body_json(StarknetRpcBaseData::call([
            serde_json::to_value(call_request).unwrap(),
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

    // Get kakarot contract bytecode
    // TODO: Use the latest mapping between starknet and EVM adresses

    mock_server.uri()
}
