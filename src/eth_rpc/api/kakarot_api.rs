use crate::providers::eth_provider::constant::Constant;
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(server, namespace = "kakarot")]
#[async_trait]
pub trait KakarotApi {
    #[method(name = "getConfig")]
    async fn get_config(&self) -> RpcResult<Constant>;
}
