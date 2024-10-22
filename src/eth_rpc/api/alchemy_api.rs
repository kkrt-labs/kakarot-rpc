use crate::models::token::{TokenBalances, TokenMetadata};
use alloy_primitives::{Address, U256};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(server, namespace = "alchemy")]
#[async_trait]
pub trait AlchemyApi {
    #[method(name = "getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> RpcResult<TokenBalances>;

    #[method(name = "getTokenMetadata")]
    async fn token_metadata(&self, contract_address: Address) -> RpcResult<TokenMetadata>;

    #[method(name = "getTokenAllowance")]
    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> RpcResult<U256>;
}
