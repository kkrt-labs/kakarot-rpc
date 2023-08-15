use std::marker::PhantomData;
use std::pin::{pin, Pin};
use std::task::Poll;

use futures::Future;
use reth_primitives::U256;
use serde::{Deserialize, Serialize};
use starknet::providers::Provider;

use crate::client::errors::EthApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAllowance {
    pub result: Option<U256>,
    pub error: Option<String>,
}

type AllowanceResult<P> = Result<U256, EthApiError<<P as Provider>::Error>>;

#[pin_project::pin_project]
pub struct FutureTokenAllowance<P: Provider, F: Future<Output = AllowanceResult<P>>> {
    #[pin]
    pub allowance: F,
    _phantom: PhantomData<P>,
}

impl<P: Provider, F: Future<Output = AllowanceResult<P>>> FutureTokenAllowance<P, F> {
    pub fn new(allowance: F) -> Self {
        Self { allowance, _phantom: PhantomData }
    }
}

impl<P: Provider, F: Future<Output = AllowanceResult<P>>> Future for FutureTokenAllowance<P, F> {
    type Output = TokenAllowance;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut this = self.project();
        let allowance = this.allowance.as_mut().poll(cx);

        match allowance {
            Poll::Ready(output) => match output {
                Ok(allowance) => Poll::Ready(TokenAllowance { result: Some(allowance), error: None }),
                Err(error) => Poll::Ready(TokenAllowance { result: None, error: Some(error.to_string()) }),
            },
            Poll::Pending => Poll::Pending,
        }
    }
}
