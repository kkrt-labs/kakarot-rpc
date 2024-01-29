use reth_rpc_types::TransactionReceipt;
use serde::Deserialize;

/// A transaction receipt as stored in the database
#[derive(Debug, Deserialize)]
pub struct StoredTransactionReceipt {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    receipt: TransactionReceipt,
}

impl From<StoredTransactionReceipt> for TransactionReceipt {
    fn from(receipt: StoredTransactionReceipt) -> Self {
        receipt.receipt
    }
}
