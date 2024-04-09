use futures::future::join_all;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Address, BlockId, BlockNumberOrTag};

use crate::eth_provider::contracts::erc20::EthereumErc20;
use crate::eth_rpc::api::alchemy_api::AlchemyApiServer;
use crate::models::balance::FutureTokenBalance;
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
    async fn token_balances(&self, address: Address, token_addresses: Vec<Address>) -> Result<TokenBalances> {
        let block_id = BlockId::Number(BlockNumberOrTag::Latest);

        let handles = token_addresses.into_iter().map(|token_addr| {
            let token = EthereumErc20::new(token_addr, &self.eth_provider);
            let balance = token.balance_of(address, block_id);

            FutureTokenBalance::new(Box::pin(balance), token_addr)
        });

        let token_balances = join_all(handles).await;

        Ok(TokenBalances { address, token_balances })
    }
}
