use reth_primitives::{AccessListItem, Address, Bytes, H256, U128, U256, U64};
use reth_rpc_types::{Parity, Signature, Transaction};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DbTransactionFull {
    pub tx: JsonRpcTransaction,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JsonRpcTransaction {
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_h256")]
    pub block_hash: H256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_h256")]
    pub block_number: H256,
    pub from: Address,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub gas: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u128")]
    pub gas_price: U128,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u128")]
    pub max_fee_per_gas: U128,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u128")]
    pub max_priority_fee_per_gas: U128,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u64")]
    pub r#type: U64,
    pub access_list: Vec<AccessListItem>,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u64")]
    pub chain_id: U64,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_h256")]
    pub hash: H256,
    pub input: Bytes,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u64")]
    pub nonce: U64,
    pub to: Option<Address>,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub transaction_index: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub value: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub v: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub r: U256,
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_u256")]
    pub s: U256,
}

impl From<JsonRpcTransaction> for Transaction {
    fn from(tx: JsonRpcTransaction) -> Self {
        let y_parity = if tx.v > U256::from(1u8) { tx.v - U256::from(35u8) } else { tx.v };
        let y_parity = Parity(y_parity.bit(0));
        Self {
            block_hash: Some(tx.block_hash),
            block_number: Some(tx.block_number.into()),
            from: tx.from,
            gas: tx.gas,
            gas_price: Some(tx.gas_price),
            max_fee_per_gas: Some(tx.max_fee_per_gas),
            max_priority_fee_per_gas: Some(tx.max_priority_fee_per_gas),
            transaction_type: Some(tx.r#type),
            access_list: Some(tx.access_list),
            chain_id: Some(tx.chain_id),
            hash: tx.hash,
            input: tx.input,
            nonce: tx.nonce,
            to: tx.to,
            transaction_index: Some(tx.transaction_index),
            value: tx.value,
            signature: Some(Signature { v: tx.v, r: tx.r, s: tx.s, y_parity: Some(y_parity) }),
            max_fee_per_blob_gas: None,
            blob_versioned_hashes: vec![],
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DbTransactionHash {
    pub tx: Hash,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub block_hash: H256,
}

impl From<Hash> for H256 {
    fn from(hash: Hash) -> Self {
        hash.block_hash
    }
}
