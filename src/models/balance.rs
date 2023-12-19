use std::marker::PhantomData;
use std::pin::{pin, Pin};
use std::task::Poll;

use futures::Future;
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};
use starknet::providers::Provider;

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

#[pin_project::pin_project]
pub struct FutureTokenBalance<P: Provider, F: Future<Output = BalanceOfResult>> {
    #[pin]
    pub balance: F,
    pub token_address: Address,
    _phantom: PhantomData<P>,
}

impl<P: Provider, F: Future<Output = BalanceOfResult>> FutureTokenBalance<P, F> {
    pub const fn new(balance: F, token_address: Address) -> Self {
        Self { balance, token_address, _phantom: PhantomData }
    }
}

impl<P: Provider, F: Future<Output = BalanceOfResult>> Future for FutureTokenBalance<P, F> {
    type Output = TokenBalance;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut this = self.project();
        let balance = this.balance.as_mut().poll(cx);
        let token_address = this.token_address.to_owned();

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
