use super::receipt::StoredTransactionReceipt;
use alloy_rpc_types::Log;
use serde::{Deserialize, Serialize};
use std::ops::Deref;

/// A transaction receipt as stored in the database
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, Serialize)]
pub struct StoredLog {
    #[serde(deserialize_with = "crate::providers::eth_provider::database::types::serde::deserialize_intermediate")]
    pub log: Log,
}

impl From<StoredLog> for Log {
    fn from(log: StoredLog) -> Self {
        log.log
    }
}

impl From<&StoredLog> for Log {
    fn from(log: &StoredLog) -> Self {
        log.log.clone()
    }
}

impl From<Log> for StoredLog {
    fn from(log: Log) -> Self {
        Self { log }
    }
}

impl From<StoredTransactionReceipt> for Vec<StoredLog> {
    fn from(value: StoredTransactionReceipt) -> Self {
        value.receipt.inner.inner.logs().iter().cloned().map(Into::into).collect()
    }
}

impl Deref for StoredLog {
    type Target = Log;

    fn deref(&self) -> &Self::Target {
        &self.log
    }
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl<'a> arbitrary::Arbitrary<'a> for StoredLog {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        Ok(Self {
            log: Log {
                block_hash: Some(alloy_primitives::B256::arbitrary(u)?),
                block_number: Some(u64::arbitrary(u)?),
                block_timestamp: Some(u64::arbitrary(u)?),
                transaction_hash: Some(alloy_primitives::B256::arbitrary(u)?),
                transaction_index: Some(u64::arbitrary(u)?),
                log_index: Some(u64::arbitrary(u)?),
                ..Log::arbitrary(u)?
            },
        })
    }
}
