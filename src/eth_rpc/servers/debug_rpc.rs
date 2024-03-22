use crate::{
    eth_provider::{
        error::{EthApiError, ReceiptError, SignatureError},
        provider::EthereumProvider,
    },
    eth_rpc::api::debug_api::DebugApiServer,
    models::{block::rpc_to_primitive_block, transaction::rpc_transaction_to_primitive},
};
use alloy_rlp::Encodable;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Bytes, Log, Receipt, ReceiptWithBloom, TransactionSigned, B256};
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
        Err(EthApiError::Unsupported("debug_rawHeader").into())
    }

    /// Returns an RLP-encoded block.
    async fn raw_block(&self, block_id: BlockId) -> Result<Bytes> {
        let block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.into(), true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        };
        let mut raw_block = Vec::new();
        if let Some(block) = block {
            let block = rpc_to_primitive_block(block.inner).map_err(EthApiError::from)?;
            block.encode(&mut raw_block);
        }
        Ok(Bytes::from(raw_block))
    }

    /// Returns a EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>> {
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(tx) = transaction {
            let mut raw_transaction = Vec::new();
            let signature = tx.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = rpc_transaction_to_primitive(tx).map_err(EthApiError::from)?;
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
        Err(EthApiError::Unsupported("debug_rawTransactions").into())
    }

    /// Returns an array of EIP-2718 binary-encoded receipts.
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        // Initializes an empty vector to store the raw receipts
        let mut raw_receipts = Vec::new();

        // Iterates through the receipts of the block using the `block_receipts` method of the Ethereum API
        for receipt in self.eth_provider.block_receipts(Some(block_id)).await?.unwrap_or_default() {
            // Converts the transaction type to a u8 and then tries to convert it into TxType
            let tx_type = match receipt.transaction_type.to::<u8>().try_into() {
                Ok(tx_type) => tx_type,
                Err(_) => return Err(EthApiError::ReceiptError(ReceiptError::ConversionError).into()),
            };

            // Tries to convert the cumulative gas used to u64
            let cumulative_gas_used = match TryInto::<u64>::try_into(receipt.cumulative_gas_used) {
                Ok(cumulative_gas_used) => cumulative_gas_used,
                Err(_) => return Err(EthApiError::ReceiptError(ReceiptError::ConversionError).into()),
            };

            // Creates a ReceiptWithBloom from the receipt data
            raw_receipts.push(
                ReceiptWithBloom {
                    receipt: Receipt {
                        tx_type,
                        success: receipt.status_code.unwrap_or_default().to::<u64>() == 1,
                        cumulative_gas_used,
                        logs: receipt
                            .logs
                            .into_iter()
                            .map(|log| Log { address: log.address, topics: log.topics, data: log.data })
                            .collect(),
                    },
                    bloom: receipt.logs_bloom,
                }
                .envelope_encoded(),
            );
        }

        // Returns the vector containing the raw receipts
        Ok(raw_receipts)
    }
}
