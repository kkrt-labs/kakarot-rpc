use reth_primitives::H256;
use reth_rpc_types::Transaction;
use serde::Deserialize;

/// A full transaction as stored in the database
#[derive(Debug, Deserialize)]
pub struct StoredTransaction {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub tx: Transaction,
}

impl From<StoredTransaction> for Transaction {
    fn from(tx: StoredTransaction) -> Self {
        tx.tx
    }
}

/// A transaction hash as stored in the database
#[derive(Debug, Deserialize)]
pub struct StoredTransactionHash {
    pub tx: Hash,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub block_hash: H256,
}

impl From<StoredTransactionHash> for H256 {
    fn from(hash: StoredTransactionHash) -> Self {
        hash.tx.block_hash
    }
}
