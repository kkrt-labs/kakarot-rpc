use jsonrpsee::core::RpcResult as Result;
use jsonrpsee::proc_macros::rpc;
use reth_primitives::{Bytes, B256};
use reth_rpc_types::BlockId;

/// Debug API
/// Taken from Reth's DebugApi trait:
/// <https://github.com/paradigmxyz/reth/blob/5d6ac4c815c562677d7ae6ad6b422b55ef4ed8e2/crates/rpc/rpc-api/src/debug.rs#L14>
#[rpc(server, namespace = "debug")]
#[async_trait]
pub trait DebugApi {
    /// Returns an RLP-encoded header.
    #[method(name = "getRawHeader")]
    async fn raw_header(&self, block_id: BlockId) -> Result<Bytes>;

    /// Returns an RLP-encoded block.
    #[method(name = "getRawBlock")]
    async fn raw_block(&self, block_id: BlockId) -> Result<Bytes>;

    /// Returns a EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    #[method(name = "getRawTransaction")]
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>>;

    /// Returns an array of EIP-2718 binary-encoded transactions for the given [BlockId].
    #[method(name = "getRawTransactions")]
    async fn raw_transactions(&self, block_id: BlockId) -> Result<Vec<Bytes>>;

    /// Returns an array of EIP-2718 binary-encoded receipts.
    #[method(name = "getRawReceipts")]
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>>;
}
