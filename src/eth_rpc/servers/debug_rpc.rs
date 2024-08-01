#![allow(clippy::blocks_in_conditions)]
use crate::{eth_rpc::api::debug_api::DebugApiServer, providers::debug_provider::provider::DebugProvider};
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Bytes, B256};
use reth_rpc_types::{
    trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TraceResult},
    BlockId, BlockNumberOrTag, TransactionRequest,
};

/// The RPC module for the implementing Net api
#[derive(Debug)]
pub struct DebugRpc<P: DebugProvider> {
    provider: P,
}

impl<P> DebugRpc<P>
where
    P: DebugProvider,
{
    pub const fn new(provider: P) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl<P> DebugApiServer for DebugRpc<P>
where
    P: DebugProvider + Send + Sync + 'static,
{
    /// Returns a RLP-encoded header.
    #[tracing::instrument(skip(self), err)]
    async fn raw_header(&self, block_id: BlockId) -> Result<Bytes> {
        self.provider.raw_header(block_id).await.map_err(Into::into)
    }

    /// Returns a RLP-encoded block.
    #[tracing::instrument(skip(self), err)]
    async fn raw_block(&self, block_id: BlockId) -> Result<Bytes> {
        self.provider.raw_block(block_id).await.map_err(Into::into)
    }

    /// Returns an EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    #[tracing::instrument(skip(self), err)]
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>> {
        self.provider.raw_transaction(hash).await.map_err(Into::into)
    }

    /// Returns an array of EIP-2718 binary-encoded transactions for the given [BlockId].
    #[tracing::instrument(skip(self), err)]
    async fn raw_transactions(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        self.provider.raw_transactions(block_id).await.map_err(Into::into)
    }

    /// Returns an array of EIP-2718 binary-encoded receipts.
    #[tracing::instrument(skip(self), err)]
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        self.provider.raw_receipts(block_id).await.map_err(Into::into)
    }

    /// Returns the Geth debug trace for the given block number.
    #[tracing::instrument(skip(self, opts), err)]
    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        self.provider.trace_block_by_number(block_number, opts).await.map_err(Into::into)
    }

    /// Returns the Geth debug trace for the given block hash.
    #[tracing::instrument(skip(self, opts), err)]
    async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        self.provider.trace_block_by_hash(block_hash, opts).await.map_err(Into::into)
    }

    /// Returns the Geth debug trace for the given transaction hash.
    #[tracing::instrument(skip(self, opts), err)]
    async fn trace_transaction(
        &self,
        transaction_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<GethTrace> {
        self.provider.trace_transaction(transaction_hash, opts).await.map_err(Into::into)
    }

    /// Runs an `eth_call` within the context of a given block execution and returns the Geth debug trace.
    #[tracing::instrument(skip(self, request, opts), err)]
    async fn trace_call(
        &self,
        request: TransactionRequest,
        block_number: Option<BlockId>,
        opts: Option<GethDebugTracingCallOptions>,
    ) -> Result<GethTrace> {
        self.provider.trace_call(request, block_number, opts).await.map_err(Into::into)
    }
}
