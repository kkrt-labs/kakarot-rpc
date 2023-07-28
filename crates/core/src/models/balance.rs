use std::pin::{pin, Pin};
use std::task::Poll;

use futures::Future;
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub contract_address: Address,
    pub token_balance: Option<U256>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

#[pin_project::pin_project]
pub struct FutureTokenBalance<F: Future> {
    #[pin]
    pub balance: F,
    pub token_address: Address,
}

impl<F: Future> Future for FutureTokenBalance<F> {
    type Output = (F::Output, Address);

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut this = self.project();
        let balance = this.balance.as_mut().poll(cx);
        let token_address = this.token_address.to_owned();

        match balance {
            Poll::Ready(output) => Poll::Ready((output, token_address)),
            Poll::Pending => Poll::Pending,
        }
    }
}
