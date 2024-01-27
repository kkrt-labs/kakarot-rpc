use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::KakarotClient;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::U64;
use reth_rpc_types::PeerCount;

use starknet::providers::Provider;
use std::sync::Arc;

use crate::eth_rpc::api::net_api::NetApiServer;

/// The RPC module for the implementing Net api
pub struct NetRpc<P: Provider + Send + Sync + 'static> {
    pub kakarot_client: Arc<KakarotClient<P>>,
}

impl<P: Provider + Send + Sync> NetRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> NetApiServer for NetRpc<P> {
    async fn version(&self) -> Result<U64> {
        // version method returns the network ID
        let chain_id = self.kakarot_client.starknet_provider().chain_id().await.map_err(EthApiError::from)?;

        let chain_id = match u64::try_from(chain_id) {
            Ok(value) => U64::from(value),
            Err(_) => {
                return Err(EthApiError::ConversionError("Conversion from Field to u64 failed".to_string()).into())
            }
        };

        Ok(chain_id)
    }

    fn peer_count(&self) -> Result<PeerCount> {
        // Kakarot RPC currently does not have peers connected to node
        Ok(PeerCount::Number(0))
    }

    fn listening(&self) -> Result<bool> {
        // Kakarot RPC currently does not support peer-to-peer connections
        Ok(false)
    }

    async fn health(&self) -> Result<bool> {
        // Calls starknet block_number method to check if it resolves
        self.kakarot_client.starknet_provider().block_number().await.map_err(EthApiError::from)?;

        Ok(true)
    }
}
