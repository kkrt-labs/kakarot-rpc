#![allow(clippy::blocks_in_conditions)]

use crate::{
    eth_rpc::api::alchemy_api::AlchemyApiServer,
    models::token::{TokenBalances, TokenMetadata},
    providers::alchemy_provider::AlchemyProvider,
};
use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use jsonrpsee::core::RpcResult as Result;

/// The RPC module for the Ethereum protocol required by Kakarot.
#[derive(Debug)]
pub struct AlchemyRpc<AP: AlchemyProvider> {
    alchemy_provider: AP,
}

impl<AP> AlchemyRpc<AP>
where
    AP: AlchemyProvider,
{
    pub const fn new(alchemy_provider: AP) -> Self {
        Self { alchemy_provider }
    }
}

#[async_trait]
impl<AP> AlchemyApiServer for AlchemyRpc<AP>
where
    AP: AlchemyProvider + Send + Sync + 'static,
{
    #[tracing::instrument(skip(self, contract_addresses), ret, err)]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        self.alchemy_provider.token_balances(address, contract_addresses).await.map_err(Into::into)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata> {
        self.alchemy_provider.token_metadata(contract_address).await.map_err(Into::into)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> Result<U256> {
        self.alchemy_provider.token_allowance(contract_address, owner, spender).await.map_err(Into::into)
    }
}
