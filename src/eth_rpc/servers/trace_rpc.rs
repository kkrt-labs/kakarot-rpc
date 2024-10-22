use crate::{
    eth_rpc::api::trace_api::TraceApiServer, providers::eth_provider::provider::EthereumProvider,
    tracing::builder::TracerBuilder,
};
use alloy_rpc_types::BlockId;
use alloy_rpc_types_trace::parity::LocalizedTransactionTrace;
use jsonrpsee::core::{async_trait, RpcResult};
use revm_inspectors::tracing::TracingInspectorConfig;
use std::sync::Arc;

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
    #[tracing::instrument(skip(self), err)]
    async fn trace_block(&self, block_id: BlockId) -> RpcResult<Option<Vec<LocalizedTransactionTrace>>> {
        tracing::info!("Serving debug_traceBlock");
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(block_id)
            .await?
            .with_tracing_options(TracingInspectorConfig::default_parity().into())
            .build()?;

        Ok(tracer.trace_block()?)
    }
}
