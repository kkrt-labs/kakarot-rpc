use reth_rpc_types::Log;
use serde::{Deserialize, Serialize};

use super::receipt::StoredTransactionReceipt;

/// A transaction receipt as stored in the database
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct StoredLog {
    pub log: Log,
}

impl From<StoredLog> for Log {
    fn from(log: StoredLog) -> Self {
        log.log
    }
}

impl From<Log> for StoredLog {
    fn from(log: Log) -> Self {
        Self { log }
    }
}

impl From<StoredTransactionReceipt> for Vec<StoredLog> {
    fn from(value: StoredTransactionReceipt) -> Self {
        value.receipt.inner.logs().iter().cloned().map(Into::into).collect()
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> arbitrary::Arbitrary<'a> for StoredLog {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            log: Log {
                block_hash: Some(reth_primitives::B256::arbitrary(u)?),
                block_number: Some(u64::arbitrary(u)?),
                block_timestamp: Some(u64::arbitrary(u)?),
                transaction_hash: Some(reth_primitives::B256::arbitrary(u)?),
                transaction_index: Some(u64::arbitrary(u)?),
                log_index: Some(u64::arbitrary(u)?),
                ..Log::arbitrary(u)?
            },
        })
    }
}
