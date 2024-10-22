use alloy_primitives::{Bytes, B256};
use jsonrpsee::{core::RpcResult, proc_macros::rpc};

#[rpc(server, namespace = "web3")]
#[async_trait]
pub trait Web3Api {
    /// Returns the client version of the running Kakarot RPC
    #[method(name = "clientVersion")]
    fn client_version(&self) -> RpcResult<String>;

    /// Returns Keccak256 of some input value
    #[method(name = "sha3")]
    fn sha3(&self, input: Bytes) -> RpcResult<B256>;
}
