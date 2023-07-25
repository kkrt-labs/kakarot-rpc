use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use kakarot_rpc_core::models::allowance::TokenAllowance;
use kakarot_rpc_core::models::balance::TokenBalances;
use kakarot_rpc_core::models::metadata::TokenMetadata;
use reth_primitives::Address;

// TODO: Define and implement of methods of Alchemy API
#[rpc(server, namespace = "alchemy")]
#[async_trait]
pub trait AlchemyApi {
    #[method(name = "getTokenAllowance")]
    async fn token_allowance(
        &self,
        contract_address: Address,
        account_address: Address,
        spender_address: Address,
    ) -> Result<TokenAllowance>;

    #[method(name = "getTokenBalances")]
    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances>;

    #[method(name = "getTokenMetadata")]
    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata>;
}
