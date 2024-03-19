use crate::eth_provider::error::EthProviderError;
use crate::eth_rpc::api::debug_api::DebugApiServer;
use crate::{eth_provider::provider::EthereumProvider, models::transaction::rpc_transaction_to_primitive};
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Bytes, Log, Receipt, TransactionSigned, B256};
use reth_rpc_types::BlockId;

/// The RPC module for the implementing Net api
pub struct DebugRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> DebugRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> DebugApiServer for DebugRpc<P> {
    /// Returns an RLP-encoded header.
    async fn raw_header(&self, _block_id: BlockId) -> Result<Bytes> {
        Err(EthProviderError::MethodNotSupported("debug_rawHeader".to_string()).into())
    }

    /// Returns an RLP-encoded block.
    async fn raw_block(&self, _block_id: BlockId) -> Result<Bytes> {
        Err(EthProviderError::MethodNotSupported("debug_rawBlock".to_string()).into())
    }

    /// Returns a EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>> {
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(tx) = transaction {
            let mut raw_transaction = Vec::new();
            let signature = tx.signature.ok_or(EthProviderError::ValueNotFound("signature".to_string()))?;
            let tx = rpc_transaction_to_primitive(tx).map_err(EthProviderError::from)?;
            TransactionSigned::from_transaction_and_signature(
                tx,
                reth_primitives::Signature {
                    r: signature.r,
                    s: signature.s,
                    odd_y_parity: signature.y_parity.unwrap_or(reth_rpc_types::Parity(false)).0,
                },
            )
            .encode_enveloped(&mut raw_transaction);
            Ok(Some(Bytes::from(raw_transaction)))
        } else {
            Ok(None)
        }
    }

    /// Returns an array of EIP-2718 binary-encoded transactions for the given [BlockId].
    async fn raw_transactions(&self, _block_id: BlockId) -> Result<Vec<Bytes>> {
        Err(EthProviderError::MethodNotSupported("debug_rawTransactions".to_string()).into())
    }

    /// Returns an array of EIP-2718 binary-encoded receipts.
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        Ok(self
            .eth_provider
            .block_receipts(Some(block_id))
            .await?
            .unwrap_or_default()
            .into_iter()
            .map(|receipt| {
                Receipt {
                    tx_type: receipt.transaction_type.to::<u8>().try_into().unwrap(),
                    success: receipt.status_code.unwrap_or_default().to::<u8>() == 1,
                    cumulative_gas_used: receipt.cumulative_gas_used.to::<u64>(),
                    logs: receipt
                        .logs
                        .into_iter()
                        .map(|log| Log { address: log.address, topics: log.topics, data: log.data })
                        .collect(),
                }
                .with_bloom()
                .envelope_encoded()
            })
            .collect())
    }
}
