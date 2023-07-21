use jsonrpsee::core::{async_trait, RpcResult as Result};
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
    /// Get the protocol version of the Kakarot Starknet RPC.
    fn version(&self) -> Result<U64> {
        let protocol_version = 1_u64;
        Ok(protocol_version.into())
    }

    fn peer_count(&self) -> Result<PeerCount> {
        todo!()
    }

    fn is_listening(&self) -> Result<bool> {
        todo!()
    }
}
