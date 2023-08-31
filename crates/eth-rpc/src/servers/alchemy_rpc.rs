use std::sync::Arc;

use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::api::KakarotEthApi;
use kakarot_rpc_core::models::allowance::TokenAllowance;
use kakarot_rpc_core::models::balance::TokenBalances;
use kakarot_rpc_core::models::metadata::TokenMetadata;
use reth_primitives::Address;
use starknet::providers::Provider;

use crate::api::alchemy_api::AlchemyApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct AlchemyRpc<P: Provider + Send + Sync> {
    pub kakarot_client: Arc<dyn KakarotEthApi<P>>,
}

impl<P: Provider + Send + Sync> AlchemyRpc<P> {
    pub fn new(kakarot_client: Arc<dyn KakarotEthApi<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> AlchemyApiServer for AlchemyRpc<P> {
    async fn token_allowance(
        &self,
        contract_address: Address,
        account_address: Address,
        spender_address: Address,
    ) -> Result<TokenAllowance> {
        let token_allowance =
            self.kakarot_client.token_allowance(contract_address, account_address, spender_address).await?;
        Ok(token_allowance)
    }

    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        let token_balances = self.kakarot_client.token_balances(address, contract_addresses).await?;
        Ok(token_balances)
    }

    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata> {
        let token_metadata = self.kakarot_client.token_metadata(contract_address).await?;
        Ok(token_metadata)
    }
}
