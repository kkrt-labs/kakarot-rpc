use std::marker::PhantomData;
use std::pin::{pin, Pin};
use std::task::Poll;

use futures::Future;
use reth_primitives::U256;
use serde::{Deserialize, Serialize};
use starknet::providers::Provider;

use crate::client::errors::EthApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub decimals: Option<U256>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub error: Option<String>,
}

type U256Result<P> = Result<U256, EthApiError<<P as Provider>::Error>>;
type StringResult<P> = Result<String, EthApiError<<P as Provider>::Error>>;

#[pin_project::pin_project]
pub struct FutureTokenMetadata<P: Provider, F: Future<Output = U256Result<P>>, G: Future<Output = StringResult<P>>> {
    #[pin]
    pub decimals: F,
    #[pin]
    pub name: G,
    #[pin]
    pub symbol: G,
    _phantom: PhantomData<P>,
}

impl<P: Provider, F: Future<Output = U256Result<P>>, G: Future<Output = StringResult<P>>> FutureTokenMetadata<P, F, G> {
    pub fn new(decimals: F, name: G, symbol: G) -> Self {
        Self { decimals, name, symbol, _phantom: PhantomData }
    }
}

impl<P: Provider, F: Future<Output = U256Result<P>>, G: Future<Output = StringResult<P>>> Future
    for FutureTokenMetadata<P, F, G>
{
    type Output = TokenMetadata;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut this = self.project();
        let decimals = this.decimals.as_mut().poll(cx);
        let name = this.name.as_mut().poll(cx);
        let symbol = this.symbol.as_mut().poll(cx);

        match (decimals, name, symbol) {
            (Poll::Ready(decimals_output), Poll::Ready(name_output), Poll::Ready(symbol_output)) => {
                match (decimals_output, name_output, symbol_output) {
                    (Ok(decimals), Ok(name), Ok(symbol)) => Poll::Ready(TokenMetadata {
                        decimals: Some(decimals),
                        name: Some(name),
                        symbol: Some(symbol),
                        error: None,
                    }),
                    (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => Poll::Ready(TokenMetadata {
                        decimals: None,
                        name: None,
                        symbol: None,
                        error: Some(error.to_string()),
                    }),
                }
            }
            (Poll::Pending, _, _) | (_, Poll::Pending, _) | (_, _, Poll::Pending) => Poll::Pending,
        }
    }
}
