use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use reth_primitives::U64;
use reth_rpc_types::PeerCount;

use crate::api::net_api::NetApiServer;

/// The RPC module for the implementing Net api
#[derive(Default)]
pub struct NetRpc {}

impl NetRpc {
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NetApiServer for NetRpc {
    fn version(&self) -> Result<U64> {
        Ok(CHAIN_ID.into())
    }

    fn peer_count(&self) -> Result<PeerCount> {
        // Kakarot RPC currently does not have peers connected to node
        Ok(PeerCount::Number(0))
    }

    fn listening(&self) -> Result<bool> {
        // Kakarot RPC currently does not support peer-to-peer connections
        Ok(false)
    }
}
