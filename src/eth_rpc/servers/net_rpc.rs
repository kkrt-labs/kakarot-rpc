use crate::{eth_rpc::api::net_api::NetApiServer, providers::eth_provider::provider::EthereumProvider};
use alloy_primitives::U64;
use jsonrpsee::core::{async_trait, RpcResult as Result};

/// The RPC module for the implementing Net api
#[derive(Debug)]
pub struct NetRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> NetRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> NetApiServer for NetRpc<P> {
    async fn version(&self) -> Result<U64> {
        Ok(self.eth_provider.chain_id().await?.unwrap_or_default())
    }

    fn peer_count(&self) -> Result<U64> {
        // Kakarot RPC currently does not have peers connected to node
        Ok(U64::ZERO)
    }

    fn listening(&self) -> Result<bool> {
        // Kakarot RPC currently does not support peer-to-peer connections
        Ok(false)
    }

    async fn health(&self) -> Result<bool> {
        // Calls starknet block_number method to check if it resolves
        let _ = self.eth_provider.block_number().await?;

        Ok(true)
    }
}
