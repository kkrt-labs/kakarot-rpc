use std::sync::Arc;

use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::trace_api::TraceApiServer;
use crate::tracing::Tracer;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_rpc_types::trace::parity::LocalizedTransactionTrace;
use reth_rpc_types::BlockId;

/// The RPC module for implementing the Trace api
#[derive(Debug)]
pub struct TraceRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> TraceRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> TraceApiServer for TraceRpc<P> {
    /// Returns the traces for the given block.
    async fn trace_block(&self, block_id: BlockId) -> Result<Option<Vec<LocalizedTransactionTrace>>> {
        let provider = Arc::new(&self.eth_provider);
        let tracer = Tracer::new(provider).await?;

        let traces = tracer.trace_block(block_id).await?;
        Ok(traces)
    }
}
