use reth_rpc_types::Header;
use serde::Deserialize;

/// A header as stored in the database
#[derive(Debug, Deserialize)]
pub struct StoredHeader {
    #[serde(deserialize_with = "crate::eth_provider::database::types::serde::deserialize_intermediate")]
    pub header: Header,
}
