use std::fmt;
use std::pin::Pin;
use std::task::Poll;

use futures::{future::BoxFuture, Future, FutureExt};
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::eth_provider::provider::EthProviderResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token_address: Address,
    pub token_balance: U256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

pub struct TokenBalanceFuture<'a> {
    pub balance: BoxFuture<'a, EthProviderResult<U256>>,
    pub token_address: Address,
}

impl<'a> fmt::Debug for TokenBalanceFuture<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TokenBalanceFuture")
            .field("balance", &"...")
            .field("token_address", &self.token_address)
            .finish()
    }
}

impl<'a> TokenBalanceFuture<'a> {
    pub fn new<F>(balance: F, token_address: Address) -> Self
    where
        F: Future<Output = EthProviderResult<U256>> + Send + 'a,
    {
        Self { balance: Box::pin(balance), token_address }
    }
}

impl<'a> Future for TokenBalanceFuture<'a> {
    type Output = EthProviderResult<TokenBalance>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let balance = self.balance.poll_unpin(cx);
        let token_address = self.token_address;

        match balance {
            Poll::Ready(output) => match output {
                Ok(token_balance) => Poll::Ready(Ok(TokenBalance { token_address, token_balance })),
                Err(err) => Poll::Ready(Err(err)),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub decimals: U256,
    pub name: String,
    pub symbol: String,
}
