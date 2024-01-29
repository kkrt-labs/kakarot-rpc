use reth_rpc_types::Log;
use serde::Deserialize;

/// A transaction receipt as stored in the database
#[derive(Debug, Deserialize)]
pub struct StoredLog {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub log: Log,
}

impl From<StoredLog> for Log {
    fn from(log: StoredLog) -> Self {
        log.log
    }
}
