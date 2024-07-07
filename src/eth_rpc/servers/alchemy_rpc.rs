#![allow(clippy::blocks_in_conditions)]

use crate::{
    eth_provider::{contracts::erc20::EthereumErc20, error::EthApiError, provider::EthereumProvider},
    eth_rpc::api::alchemy_api::AlchemyApiServer,
    models::token::{TokenBalance, TokenBalances, TokenMetadata},
};
use futures::future::join_all;
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};

/// The RPC module for the Ethereum protocol required by Kakarot.
#[derive(Debug)]
pub struct AlchemyRpc<P: EthereumProvider> {
    /// The provider for interacting with the Ethereum network.
    eth_provider: P,
}

impl<P: EthereumProvider> AlchemyRpc<P> {
    /// Creates a new instance of [`AlchemyRpc`].
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> AlchemyApiServer for AlchemyRpc<P> {
    /// Retrieves the token balances for a given address.
    #[tracing::instrument(skip_all, ret, fields(address = %address), err)]
    async fn token_balances(&self, address: Address, token_addresses: Vec<Address>) -> RpcResult<TokenBalances> {
        tracing::info!("Serving alchemy_getTokenBalances");

        // Set the block ID to the latest block
        let block_id = BlockId::Number(BlockNumberOrTag::Latest);

        Ok(TokenBalances {
            address,
            token_balances: join_all(token_addresses.into_iter().map(|token_address| async move {
                // Create a new instance of `EthereumErc20` for each token address
                let token = EthereumErc20::new(token_address, &self.eth_provider);
                // Retrieve the balance for the given address
                let token_balance = token.balance_of(address, block_id).await?;
                Ok(TokenBalance { token_address, token_balance })
            }))
            .await
            .into_iter()
            .collect::<Result<Vec<_>, EthApiError>>()?,
        })
    }

    /// Retrieves the metadata for a given token.
    #[tracing::instrument(skip(self), ret, err)]
    async fn token_metadata(&self, token_address: Address) -> RpcResult<TokenMetadata> {
        tracing::info!("Serving alchemy_getTokenMetadata");

        // Set the block ID to the latest block
        let block_id = BlockId::Number(BlockNumberOrTag::Latest);
        // Create a new instance of `EthereumErc20`
        let token = EthereumErc20::new(token_address, &self.eth_provider);

        // Await all futures concurrently to retrieve decimals, name, and symbol
        let (decimals, name, symbol) =
            futures::try_join!(token.decimals(block_id), token.name(block_id), token.symbol(block_id))?;

        // Return the metadata
        Ok(TokenMetadata { decimals, name, symbol })
    }

    /// Retrieves the allowance of a given owner for a spender.
    #[tracing::instrument(skip(self), ret, err)]
    async fn token_allowance(&self, token_address: Address, owner: Address, spender: Address) -> RpcResult<U256> {
        tracing::info!("Serving alchemy_getTokenAllowance");

        // Set the block ID to the latest block
        let block_id = BlockId::Number(BlockNumberOrTag::Latest);
        // Create a new instance of `EthereumErc20`
        let token = EthereumErc20::new(token_address, &self.eth_provider);
        // Retrieve the allowance for the given owner and spender
        let allowance = token.allowance(owner, spender, block_id).await?;

        // Return the allowance
        Ok(allowance)
    }
}
