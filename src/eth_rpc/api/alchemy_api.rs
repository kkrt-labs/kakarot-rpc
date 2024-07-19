use crate::models::token::{TokenBalances, TokenMetadata};
use jsonrpsee::{core::RpcResult as Result, proc_macros::rpc};
use reth_primitives::{Address, U256};

#[rpc(server, namespace = "alchemy")]
#[async_trait]
pub trait AlchemyApi {
    #[method(name = "getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances>;

    #[method(name = "getTokenMetadata")]
    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata>;

    #[method(name = "getTokenAllowance")]
    async fn token_allowance(&self, contract_address: Address, owner: Address, spender: Address) -> Result<U256>;
}
