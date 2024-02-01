use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::U64;
use reth_rpc_types::PeerCount;

// TODO: Define and implement of methods of Net API
#[rpc(server, namespace = "net")]
#[async_trait]
pub trait NetApi {
    /// Returns the protocol version encoded as a string.
    #[method(name = "version")]
    async fn version(&self) -> Result<U64>;

    /// Returns number of peers connected to node.
    #[method(name = "peerCount")]
    fn peer_count(&self) -> Result<PeerCount>;

    /// Returns true if client is actively listening for network connections.
    /// Otherwise false.
    #[method(name = "listening")]
    fn listening(&self) -> Result<bool>;

    /// Returns true if Kakarot RPC_URL is reachable.
    /// Otherwise throw an EthApiError.
    #[method(name = "health")]
    async fn health(&self) -> Result<bool>;
}
