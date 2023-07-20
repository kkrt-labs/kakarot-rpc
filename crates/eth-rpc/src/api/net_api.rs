use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::U64;

// TODO: Define and implement of methods of Net API
#[rpc(server, namespace = "net")]
#[async_trait]
pub trait NetApi {
    /// Returns the protocol version encoded as a string.
    #[method(name = "version")]
    fn protocol_version(&self) -> Result<U64>;
}
