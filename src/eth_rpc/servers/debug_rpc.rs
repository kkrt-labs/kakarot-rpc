#![allow(clippy::blocks_in_conditions)]
use std::sync::Arc;

use alloy_rlp::Encodable;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Block, Bytes, Header, Log, Receipt, ReceiptWithBloom, TransactionSigned, B256};
use reth_rpc_types::trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TraceResult};
use reth_rpc_types::{BlockId, BlockNumberOrTag, TransactionRequest};

use crate::eth_provider::error::{EthApiError, EthereumDataFormatError, SignatureError};
use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::debug_api::DebugApiServer;
use crate::tracing::builder::TracerBuilder;

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
    #[tracing::instrument(skip(self), err, fields(block_id = ?block_id))]
    async fn raw_header(&self, block_id: BlockId) -> Result<Bytes> {
        tracing::info!("Serving debug_getRawHeader");

        let mut res = Vec::new();
        if let Some(header) = self
            .eth_provider
            .header(&block_id)
            .await?
            .map(Header::try_from)
            .transpose()
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::HeaderConversion))?
        {
            header.encode(&mut res);
        }

        Ok(res.into())
    }

    /// Returns an RLP-encoded block.
    #[tracing::instrument(skip(self), err, fields(block_id = ?block_id))]
    async fn raw_block(&self, block_id: BlockId) -> Result<Bytes> {
        tracing::info!("Serving debug_getRawBlock");

        let block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.into(), true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        };
        let mut raw_block = Vec::new();
        if let Some(block) = block {
            let block =
                Block::try_from(block.inner).map_err(|_| EthApiError::from(EthereumDataFormatError::Primitive))?;
            block.encode(&mut raw_block);
        }
        Ok(raw_block.into())
    }

    /// Returns a EIP-2718 binary-encoded transaction.
    ///
    /// If this is a pooled EIP-4844 transaction, the blob sidecar is included.
    #[tracing::instrument(skip(self), err, fields(hash = ?hash))]
    async fn raw_transaction(&self, hash: B256) -> Result<Option<Bytes>> {
        tracing::info!("Serving debug_getRawTransaction");

        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(tx) = transaction {
            let signature = tx.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = tx.try_into().map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::Primitive))?;
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
    #[tracing::instrument(skip(self), err, fields(block_id = ?block_id))]
    async fn raw_transactions(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        tracing::info!("Serving debug_getRawTransactions");

        let transactions = self.eth_provider.block_transactions(Some(block_id)).await?.unwrap_or_default();
        let mut raw_transactions = Vec::with_capacity(transactions.len());

        for t in transactions {
            let signature = t.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = t.try_into().map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::Primitive))?;
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
    #[tracing::instrument(skip(self), err, fields(block_id = ?block_id))]
    async fn raw_receipts(&self, block_id: BlockId) -> Result<Vec<Bytes>> {
        tracing::info!("Serving debug_getRawReceipts");

        let receipts = self.eth_provider.block_receipts(Some(block_id)).await?.unwrap_or_default();

        // Initializes an empty vector to store the raw receipts
        let mut raw_receipts = Vec::with_capacity(receipts.len());

        // Iterates through the receipts of the block using the `block_receipts` method of the Ethereum API
        for receipt in receipts {
            // Converts the transaction type to a u8 and then tries to convert it into TxType
            let tx_type = Into::<u8>::into(receipt.transaction_type())
                .try_into()
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::ReceiptConversion))?;

            // Tries to convert the cumulative gas used to u64
            let cumulative_gas_used = TryInto::<u64>::try_into(receipt.inner.cumulative_gas_used())
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::ReceiptConversion))?;

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

    /// Returns the Geth debug trace for the given block number.
    #[tracing::instrument(skip(self), err, fields(block_number = ?block_number, opts = ?opts))]
    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        tracing::info!("Serving debug_traceBlockByNumber");

        let provider = Arc::new(&self.eth_provider);
        let tracer = TracerBuilder::new(provider)
            .await?
            .with_block_id(BlockId::Number(block_number))
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_block()?)
    }

    /// Returns the Geth debug trace for the given block hash.
    #[tracing::instrument(skip(self), err, fields(block_hash = ?block_hash, opts = ?opts))]
    async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<Vec<TraceResult>> {
        tracing::info!("Serving debug_traceBlockByHash");

        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(BlockId::Hash(block_hash.into()))
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_block()?)
    }

    /// Returns the Geth debug trace for the given transaction hash.
    #[tracing::instrument(skip(self), err, fields(transaction_hash = ?transaction_hash, opts = ?opts))]
    async fn trace_transaction(
        &self,
        transaction_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> Result<GethTrace> {
        tracing::info!("Serving debug_traceTransaction");

        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_transaction_hash(transaction_hash)
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_transaction(transaction_hash)?)
    }

    /// Runs an `eth_call` within the context of a given block execution and returns the Geth debug trace.
    #[tracing::instrument(skip(self), err, fields(request = ?request, block_number = ?block_number, opts=?opts))]
    async fn trace_call(
        &self,
        request: TransactionRequest,
        block_number: Option<BlockId>,
        opts: Option<GethDebugTracingCallOptions>,
    ) -> Result<GethTrace> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(block_number.unwrap_or_default())
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_transaction_request(request)?)
    }
}
