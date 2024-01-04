use crate::starknet_client::constants::KAKAROT_CLIENT_VERSION;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{keccak256, Bytes, H256};

use crate::eth_rpc::api::web3_api::Web3ApiServer;

/// The RPC module for the implementing Web3 Api { i.e rpc endpoints prefixed with web3_ }
#[derive(Default)]
pub struct Web3Rpc {}

impl Web3Rpc {
    pub const fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl Web3ApiServer for Web3Rpc {
    fn client_version(&self) -> Result<String> {
        Ok((*KAKAROT_CLIENT_VERSION).clone())
    }

    fn sha3(&self, input: Bytes) -> Result<H256> {
        let hash = keccak256(input);
        Ok(hash)
    }
}
