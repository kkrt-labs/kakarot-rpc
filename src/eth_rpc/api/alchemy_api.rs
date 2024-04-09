use crate::models::balance::TokenBalances;
use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::Address;

/// TODO: Define and implement of methods of Alchemy API
/// Represents the Alchemy API.
#[rpc(server, namespace = "alchemy")]
#[async_trait]
pub trait AlchemyApi {
    /// Asynchronously retrieves token balances for the given address and contract addresses.
    #[method(name = "getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances>;
}
