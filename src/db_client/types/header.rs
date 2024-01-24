use reth_rpc_types::Header;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct DbHeader {
    #[serde(deserialize_with = "crate::db_client::types::serde::deserialize_intermediate")]
    pub header: Header,
}
