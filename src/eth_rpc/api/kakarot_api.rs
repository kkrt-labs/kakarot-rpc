use jsonrpsee::{core::RpcResult as Result, proc_macros::rpc};
use reth_primitives::B256;

#[rpc(server, namespace = "kakarot")]
#[async_trait]
pub trait KakarotApi {
    #[method(name = "getStarknetTransactionHash")]
    async fn kakarot_get_starknet_transaction_hash(&self, hash: B256, retries: u8) -> Result<B256>;
}
