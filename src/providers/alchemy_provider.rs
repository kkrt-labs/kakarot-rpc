use crate::{
    models::token::{TokenBalance, TokenBalances, TokenMetadata},
    providers::eth_provider::{
        contracts::erc20::EthereumErc20,
        error::EthApiError,
        provider::{EthApiResult, EthereumProvider},
    },
};
use alloy_primitives::{Address, U256};
use async_trait::async_trait;
use auto_impl::auto_impl;
use eyre::Result;
use futures::future::join_all;
use mongodb::bson::doc;
use reth_primitives::BlockNumberOrTag;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait AlchemyProvider {
    /// Retrieves the token balances for a given address.
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> EthApiResult<TokenBalances>;
    /// Retrieves the metadata for a given token.
    async fn token_metadata(&self, contract_address: Address) -> EthApiResult<TokenMetadata>;
    /// Retrieves the allowance for a given token.
    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> EthApiResult<U256>;
}

#[derive(Debug, Clone)]
pub struct AlchemyDataProvider<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> AlchemyDataProvider<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> AlchemyProvider for AlchemyDataProvider<P> {
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> EthApiResult<TokenBalances> {
        // Set the block ID to the latest block
        let block_id = BlockNumberOrTag::Latest.into();

        Ok(TokenBalances {
            address,
            token_balances: join_all(contract_addresses.into_iter().map(|token_address| async move {
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
    async fn token_metadata(&self, contract_address: Address) -> EthApiResult<TokenMetadata> {
        // Set the block ID to the latest block
        let block_id = BlockNumberOrTag::Latest.into();
        // Create a new instance of `EthereumErc20`
        let token = EthereumErc20::new(contract_address, &self.eth_provider);

        // Await all futures concurrently to retrieve decimals, name, and symbol
        let (decimals, name, symbol) =
            futures::try_join!(token.decimals(block_id), token.name(block_id), token.symbol(block_id))?;

        // Return the metadata
        Ok(TokenMetadata { decimals, name, symbol })
    }

    /// Retrieves the allowance of a given owner for a spender.
    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> EthApiResult<U256> {
        // Set the block ID to the latest block
        let block_id = BlockNumberOrTag::Latest.into();
        // Create a new instance of `EthereumErc20`
        let token = EthereumErc20::new(contract_address, &self.eth_provider);
        // Retrieve the allowance for the given owner and spender
        let allowance = token.allowance(owner, spender, block_id).await?;

        // Return the allowance
        Ok(allowance)
    }
}
