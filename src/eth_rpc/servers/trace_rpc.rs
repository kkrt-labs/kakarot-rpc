use std::sync::Arc;

use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::trace_api::TraceApiServer;
use crate::tracing::builder::TracerBuilder;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_rpc_types::trace::parity::LocalizedTransactionTrace;
use reth_rpc_types::BlockId;
use revm_inspectors::tracing::TracingInspectorConfig;

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
    /// Returns the parity traces for the given block.
    #[allow(clippy::blocks_in_conditions)]
    #[tracing::instrument(skip(self), err, fields(block_id = ?block_id))]
    async fn trace_block(&self, block_id: BlockId) -> Result<Option<Vec<LocalizedTransactionTrace>>> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(block_id)
            .await?
            .with_tracing_options(TracingInspectorConfig::default_parity().into())
            .build()?;

        Ok(tracer.trace_block()?)
    }
}
