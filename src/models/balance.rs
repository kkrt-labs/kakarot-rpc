use std::pin::Pin;
use std::task::Poll;

use futures::{Future, FutureExt};
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::starknet_client::errors::EthApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub token_address: Address,
    pub token_balance: Option<U256>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

type BalanceOfResult = Result<U256, EthApiError>;

pub struct FutureTokenBalance<F: Future<Output = BalanceOfResult>> {
    pub balance: F,
    pub token_address: Address,
}

impl<F: Future<Output = BalanceOfResult>> FutureTokenBalance<F> {
    pub const fn new(balance: F, token_address: Address) -> Self {
        Self { balance, token_address }
    }
}

impl<F: Future<Output = BalanceOfResult> + Unpin> Future for FutureTokenBalance<F> {
    type Output = TokenBalance;

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let balance = self.balance.poll_unpin(cx);
        let token_address = self.token_address.to_owned();

        match balance {
            Poll::Ready(output) => match output {
                Ok(balance) => Poll::Ready(TokenBalance { token_address, token_balance: Some(balance), error: None }),
                Err(error) => {
                    Poll::Ready(TokenBalance { token_address, token_balance: None, error: Some(error.to_string()) })
                }
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
