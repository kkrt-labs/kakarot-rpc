use reth_primitives::{Address, Bloom, Bytes, H256, U256, U64};
use reth_rpc_types::Header;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DbHeader {
    pub header: JsonRpcHeader,
    pub hash: H256,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcHeader {
    pub parent_hash: H256,
    pub uncle_hash: H256,
    pub coinbase: Address,
    pub state_root: H256,
    pub transactions_trie: H256,
    pub receipt_trie: H256,
    pub logs_bloom: Bloom,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub difficulty: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u64")]
    pub number: U64,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub gas_limit: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub gas_used: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub timestamp: U256,
    pub extra_data: Bytes,
    pub mix_hash: H256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub base_fee_per_gas: U256,
}

impl From<DbHeader> for Header {
    fn from(value: DbHeader) -> Self {
        Self {
            hash: Some(value.hash),
            parent_hash: value.header.parent_hash,
            uncles_hash: value.header.uncle_hash,
            state_root: value.header.state_root,
            transactions_root: value.header.transactions_trie,
            receipts_root: value.header.receipt_trie,
            logs_bloom: value.header.logs_bloom,
            difficulty: value.header.difficulty,
            number: Some(U256::from_limbs_slice(&value.header.number.0)),
            gas_limit: value.header.gas_limit,
            gas_used: value.header.gas_used,
            timestamp: value.header.timestamp,
            extra_data: value.header.extra_data,
            mix_hash: value.header.mix_hash,
            base_fee_per_gas: Some(value.header.base_fee_per_gas),
            miner: value.header.coinbase,
            nonce: None,
            withdrawals_root: None,
            blob_gas_used: None,
            excess_blob_gas: None,
            parent_beacon_block_root: None,
        }
    }
}
