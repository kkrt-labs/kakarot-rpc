use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::constants::{CHAIN_ID, PEER_COUNT};
use reth_primitives::U64;
use reth_rpc_types::PeerCount;

use crate::api::net_api::NetApiServer;

/// The RPC module for the implementing Net api
#[derive(Default)]
pub struct NetRpc {}

impl NetRpc {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NetApiServer for NetRpc {
    fn version(&self) -> Result<U64> {
        Ok(CHAIN_ID.into())
    }

    fn peer_count(&self) -> Result<PeerCount> {
        Ok(*PEER_COUNT)
    }

    fn listening(&self) -> Result<bool> {
        Ok(false)
    }
}
