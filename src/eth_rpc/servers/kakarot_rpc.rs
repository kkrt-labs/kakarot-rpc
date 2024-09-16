use crate::{
    config::KakarotRpcConfig,
    eth_rpc::api::kakarot_api::KakarotApiServer,
    providers::eth_provider::{
        constant::{Constant, MAX_LOGS},
        starknet::kakarot_core::{get_white_listed_eip_155_transaction_hashes, MAX_FELTS_IN_CALLDATA},
    },
};
use jsonrpsee::core::{async_trait, RpcResult};

#[derive(Debug)]
pub struct KakarotRpc;

#[async_trait]
impl KakarotApiServer for KakarotRpc {
    async fn get_config(&self) -> RpcResult<Constant> {
        let starknet_config = KakarotRpcConfig::from_env().expect("Failed to load Kakarot RPC config");
        Ok(Constant {
            max_logs: *MAX_LOGS,
            starknet_network: String::from(starknet_config.network_url),
            max_felts_in_calldata: *MAX_FELTS_IN_CALLDATA,
            white_listed_eip_155_transaction_hashes: get_white_listed_eip_155_transaction_hashes(),
        })
    }
}
