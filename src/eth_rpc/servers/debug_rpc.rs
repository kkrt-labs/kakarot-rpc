use crate::eth_provider::error::{EthApiError, EthereumDataFormatError, SignatureError};
use crate::eth_rpc::api::debug_api::DebugApiServer;
use crate::{eth_provider::provider::EthereumProvider, models::transaction::rpc_to_primitive_transaction};
use alloy_rlp::Encodable;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Block, Bytes, Header, Log, Receipt, ReceiptWithBloom, TransactionSigned, B256};
use reth_rpc_types::BlockId;

/// The RPC module for the implementing Net api
#[derive(Debug)]
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
    async fn raw_header(&self, block_id: BlockId) -> Result<Bytes> {
        let mut res = Vec::new();
        if let Some(header) = self
            .eth_provider
            .header(&block_id)
            .await?
            .map(Header::try_from)
            .transpose()
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::HeaderConversionError))?
        {
            header.encode(&mut res);
        }

        Ok(res.into())
    }

    /// Returns an RLP-encoded block.
    async fn raw_block(&self, block_id: BlockId) -> Result<Bytes> {
        let block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.into(), true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        };
        let mut raw_block = Vec::new();
        if let Some(block) = block {
            let block =
                Block::try_from(block.inner).map_err(|_| EthApiError::from(EthereumDataFormatError::PrimitiveError))?;
            block.encode(&mut raw_block);
        }
        Ok(raw_block.into())
    }

    /// Returns a EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>> {
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(tx) = transaction {
            let signature = tx.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = rpc_to_primitive_transaction(tx).map_err(EthApiError::from)?;
            let bytes = TransactionSigned::from_transaction_and_signature(
                tx,
                reth_primitives::Signature {
                    r: signature.r,
                    s: signature.s,
                    odd_y_parity: signature.y_parity.unwrap_or(reth_rpc_types::Parity(false)).0,
                },
            )
            .envelope_encoded();
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// Returns an array of EIP-2718 binary-encoded transactions for the given [BlockId].
    async fn raw_transactions(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        let transactions = self.eth_provider.block_transactions(Some(block_id)).await?.unwrap_or_default();
        let mut raw_transactions = Vec::with_capacity(transactions.len());

        for t in transactions {
            let signature = t.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = rpc_to_primitive_transaction(t).map_err(EthApiError::from)?;
            let bytes = TransactionSigned::from_transaction_and_signature(
                tx,
                reth_primitives::Signature {
                    r: signature.r,
                    s: signature.s,
                    odd_y_parity: signature.y_parity.unwrap_or(reth_rpc_types::Parity(false)).0,
                },
            )
            .envelope_encoded();
            raw_transactions.push(bytes);
        }

        Ok(raw_transactions)
    }

    /// Returns an array of EIP-2718 binary-encoded receipts.
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        let receipts = self.eth_provider.block_receipts(Some(block_id)).await?.unwrap_or_default();

        // Initializes an empty vector to store the raw receipts
        let mut raw_receipts = Vec::with_capacity(receipts.len());

        // Iterates through the receipts of the block using the `block_receipts` method of the Ethereum API
        for receipt in receipts {
            // Converts the transaction type to a u8 and then tries to convert it into TxType
            let tx_type = Into::<u8>::into(receipt.transaction_type())
                .try_into()
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::ReceiptConversionError))?;

            // Tries to convert the cumulative gas used to u64
            let cumulative_gas_used = TryInto::<u64>::try_into(receipt.inner.cumulative_gas_used())
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::ReceiptConversionError))?;

            // Creates a ReceiptWithBloom from the receipt data
            raw_receipts.push(
                ReceiptWithBloom {
                    receipt: Receipt {
                        tx_type,
                        success: receipt.inner.status(),
                        cumulative_gas_used,
                        logs: receipt
                            .inner
                            .logs()
                            .iter()
                            .filter_map(|log| Log::new(log.address(), log.topics().to_vec(), log.data().data.clone()))
                            .collect(),
                    },
                    bloom: *receipt.inner.logs_bloom(),
                }
                .envelope_encoded(),
            );
        }

        // Returns the vector containing the raw receipts
        Ok(raw_receipts)
    }
}
