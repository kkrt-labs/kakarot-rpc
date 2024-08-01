#![allow(clippy::blocks_in_conditions)]

use crate::{
    alchemy_provider::provider::AlchemyProvider,
    eth_rpc::api::alchemy_api::AlchemyApiServer,
    models::token::{TokenBalances, TokenMetadata},
};
use async_trait::async_trait;
use jsonrpsee::core::RpcResult as Result;
use reth_primitives::{Address, U256};

pub struct AlchemyRpc<P: AlchemyProvider> {
    provider: P,
}

impl<P> AlchemyRpc<P>
where
    P: AlchemyProvider,
{
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl<P> AlchemyApiServer for AlchemyRpc<P>
where
    P: AlchemyProvider + Send + Sync + 'static,
{
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        self.provider.token_balances(address, contract_addresses).await.map_err(Into::into)
    }

    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata> {
        self.provider.token_metadata(contract_address).await.map_err(Into::into)
    }

    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> Result<U256> {
        self.provider.token_allowance(contract_address, owner, spender).await.map_err(Into::into)
    }
}
