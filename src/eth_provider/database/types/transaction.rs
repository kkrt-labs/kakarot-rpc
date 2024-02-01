use reth_primitives::B256;
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
/// This wrapper is used to deserialize a transaction
/// from the database, on which a projection was
/// performed in order to only return the transaction
/// hash (e.g. {tx: {hash: "0x1234"}})
#[derive(Debug, Deserialize)]
pub struct StoredTransactionHash {
    #[serde(rename = "tx")]
    pub tx_hash: Hash,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hash {
    pub hash: B256,
}

impl From<StoredTransactionHash> for B256 {
    fn from(hash: StoredTransactionHash) -> Self {
        hash.tx_hash.hash
    }
}
