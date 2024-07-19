use jsonrpsee::{core::RpcResult as Result, proc_macros::rpc};
use reth_rpc_types::{trace::parity::LocalizedTransactionTrace, BlockId};

/// Trace API
#[rpc(server, namespace = "trace")]
#[async_trait]
pub trait TraceApi {
    /// Returns the parity traces for the given block.
    #[method(name = "block")]
    async fn trace_block(&self, block_id: BlockId) -> Result<Option<Vec<LocalizedTransactionTrace>>>;
}
