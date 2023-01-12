use jsonrpsee::core::{async_trait, RpcResult};
use jsonrpsee::proc_macros::rpc;
use kakarot_rpc_core::lightclient::StarknetClient;
// use reth_primitives::{
//     rpc::{transaction::eip2930::AccessListWithGasUsed, BlockId},
//     Address, BlockNumber, Bytes, H256, H64, U256, U64,
// };
// use reth_rpc_api::EthApiServer;
// use reth_rpc_types::{
//     CallRequest, EIP1186AccountProofResponse, FeeHistory, Index, RichBlock, SyncStatus,
//     TransactionReceipt, TransactionRequest, Work,
// };

/// The RPC module for the Ethereum protocol required by Kakarot.
///
///
pub struct KakarotEthRpc {
    pub starknet_client: StarknetClient,
}

#[rpc(server, client)]
trait EthApi {
    #[method(name = "eth_blockNumber")]
    async fn get_block_number(&self) -> RpcResult<u64>;
}

#[async_trait]
impl EthApiServer for KakarotEthRpc {
    async fn get_block_number(&self) -> RpcResult<u64> {
        let block_number = self
            .starknet_client
            .block_number()
            .await
            .map_err(|e| eyre::eyre!(e))
            .unwrap();
        Ok(block_number)
    }
}
