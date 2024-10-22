use crate::eth_rpc::api::web3_api::Web3ApiServer;
use alloy_primitives::{keccak256, Bytes, B256};
use jsonrpsee::core::{async_trait, RpcResult};

/// The RPC module for the implementing Web3 Api { i.e rpc endpoints prefixed with web3_ }
#[derive(Default, Debug)]
pub struct Web3Rpc {}

impl Web3Rpc {
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Web3ApiServer for Web3Rpc {
    fn client_version(&self) -> RpcResult<String> {
        Ok(format!("kakarot_{}", env!("CARGO_PKG_VERSION")))
    }

    fn sha3(&self, input: Bytes) -> RpcResult<B256> {
        Ok(keccak256(input))
    }
}
