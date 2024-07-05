use futures::future::join_all;
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::{Address, BlockId, BlockNumberOrTag};

use crate::eth_provider::contracts::erc20::EthereumErc20;
use crate::eth_provider::error::EthApiError;
use crate::eth_rpc::api::alchemy_api::AlchemyApiServer;
use crate::models::balance::TokenBalanceFuture;
use crate::{eth_provider::provider::EthereumProvider, models::balance::TokenBalances};

/// The RPC module for the Ethereum protocol required by Kakarot.
#[derive(Debug)]
pub struct AlchemyRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> AlchemyRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> AlchemyApiServer for AlchemyRpc<P> {
    #[tracing::instrument(skip_all, ret, fields(address = %address, token_addresses = ?token_addresses))]
    async fn token_balances(&self, address: Address, token_addresses: Vec<Address>) -> RpcResult<TokenBalances> {
        tracing::info!("Serving alchemy_getTokenBalances");

        let block_id = BlockId::Number(BlockNumberOrTag::Latest);
        let handles = token_addresses.into_iter().map(|token_addr| {
            let token = EthereumErc20::new(token_addr, &self.eth_provider);
            let balance = token.balance_of(address, block_id);

            TokenBalanceFuture::new(Box::pin(balance), token_addr)
        });

        let token_balances = join_all(handles).await.into_iter().collect::<Result<Vec<_>, EthApiError>>()?;

        Ok(TokenBalances { address, token_balances })
    }
}
