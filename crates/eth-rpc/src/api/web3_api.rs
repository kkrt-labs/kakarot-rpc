use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::{Bytes, H256};

#[rpc(server)]
#[async_trait]
pub trait Web3Api {
    /// Returns the client version of the running Kakarot RPC
    #[method(name = "web3_clientVersion")]
    fn client_version(&self) -> Result<String>;

    /// Returns Keccak256 of some input value
    #[method(name = "web3_sha3")]
    fn sha3(&self, input: Bytes) -> Result<H256>;
}
