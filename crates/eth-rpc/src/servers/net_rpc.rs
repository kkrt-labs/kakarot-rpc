use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::constants::CHAIN_ID;
use kakarot_rpc_core::client::errors::EthApiError;
use kakarot_rpc_core::client::KakarotClient;
use reth_primitives::U64;
use reth_rpc_types::PeerCount;

use starknet::providers::Provider;
use std::sync::Arc;

use crate::api::net_api::NetApiServer;

/// The RPC module for the implementing Net api
pub struct NetRpc<P: Provider + Send + Sync + 'static> {
    pub kakarot_client: Arc<KakarotClient<P>>,
}

impl<P: Provider + Send + Sync> NetRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>) -> Self {
        Self { kakarot_client }
impl<P: Provider + Send + Sync> NetRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> NetApiServer for NetRpc<P> {
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

    async fn health(&self) -> Result<bool> {
        // Calls starknet block_number method to check if it resolves
        self.kakarot_client.starknet_provider().block_number().await.map_err(EthApiError::from)?;

        Ok(true)
    }
}
