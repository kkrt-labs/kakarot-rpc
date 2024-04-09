use std::pin::Pin;
use std::task::Poll;

use futures::{Future, FutureExt};
use reth_primitives::{Address, U256};
use serde::{Deserialize, Serialize};

use crate::eth_provider::error::EthApiError;

/// Represents the balance of a token associated with a specific address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    /// The address of the token.
    pub token_address: Address,
    /// The balance of the token.
    pub token_balance: Option<U256>,
    /// Any error message associated with retrieving the balance.
    pub error: Option<String>,
}

/// Represents the balances of multiple tokens associated with a specific address.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    /// The address for which token balances are being tracked.
    pub address: Address,
    /// A vector containing the balances of multiple tokens.
    pub token_balances: Vec<TokenBalance>,
}

/// Alias for the result of querying the balance of a token.
type BalanceOfResult = Result<U256, EthApiError>;

/// Represents a future token balance computation.
#[derive(Debug)]
pub struct FutureTokenBalance<F: Future<Output = BalanceOfResult>> {
    /// The future computation of the token balance.
    pub balance: F,
    /// The address of the token.
    pub token_address: Address,
}

impl<F: Future<Output = BalanceOfResult>> FutureTokenBalance<F> {
    /// Creates a new instance of [`FutureTokenBalance`].
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
