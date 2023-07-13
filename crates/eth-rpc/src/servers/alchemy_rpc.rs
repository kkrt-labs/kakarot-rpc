use std::sync::Arc;

use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::api::KakarotEthApi;
use kakarot_rpc_core::models::balance::TokenBalances;
use reth_primitives::Address;
use starknet::providers::Provider;

use crate::api::alchemy_api::AlchemyApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct AlchemyRpc<P: Provider + Send + Sync> {
    pub kakarot_client: Arc<dyn KakarotEthApi<P>>,
}

impl<P: Provider + Send + Sync> AlchemyRpc<P> {
    #[must_use]
    pub fn new(kakarot_client: Arc<dyn KakarotEthApi<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> AlchemyApiServer for AlchemyRpc<P> {
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        let token_balances = self.kakarot_client.token_balances(address, contract_addresses).await?;
        Ok(token_balances)
    }
}
