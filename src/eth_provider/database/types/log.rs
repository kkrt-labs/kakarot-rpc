use reth_rpc_types::Log;
use serde::{Deserialize, Serialize};

use super::receipt::StoredTransactionReceipt;

/// A transaction receipt as stored in the database
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct StoredLog {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub log: Log,
}

impl From<StoredLog> for Log {
    fn from(log: StoredLog) -> Self {
        log.log
    }
}

impl From<Log> for StoredLog {
    fn from(log: Log) -> Self {
        StoredLog { log }
    }
}

impl From<StoredTransactionReceipt> for Vec<StoredLog> {
    fn from(value: StoredTransactionReceipt) -> Self {
        value.receipt.inner.logs().iter().cloned().map(Into::<StoredLog>::into).collect()
    }
}
