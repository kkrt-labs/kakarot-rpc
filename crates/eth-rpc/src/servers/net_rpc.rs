use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use reth_primitives::U64;
use reth_rpc_types::PeerCount;
use std::io;

use crate::api::net_api::NetApiServer;

/// The RPC module for the implementing Net api
#[derive(Default)]
pub struct NetRpc<P: Provider + Send + Sync> {
    pub kakarot_client: Arc<KakarotClient<P>>,
}

impl<P: Provider + Send + Sync> NetRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>) -> Self {
        Self { kakarot_client }
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

    fn health(&self) -> Result<bool, io::Error> {
        // call `starknet_blockNumber` function to check if it resolves
        match self.kakarot_client.starknet_blockNumber() {
            Ok(_) => Ok(true),
            Err(_) => Err(io::Error::new(io::ErrorKind::NotFound, "Kakarot RPC currently unreacheable")),
        }
    }
}
