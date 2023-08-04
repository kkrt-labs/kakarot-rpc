use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use kakarot_rpc_core::models::balance::TokenBalances;
use reth_primitives::Address;

// TODO: Define and implement of methods of Alchemy API
#[rpc(server, namespace = "alchemy")]
#[async_trait]
pub trait AlchemyApi {
    #[method(name = "getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances>;
}
