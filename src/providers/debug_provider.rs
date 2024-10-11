use crate::{
    providers::eth_provider::{
        error::{EthApiError, EthereumDataFormatError, SignatureError},
        provider::{EthApiResult, EthereumProvider},
    },
    tracing::builder::TracerBuilder,
};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Bytes, B256};
use alloy_rlp::Encodable;
use alloy_rpc_types::TransactionRequest;
use alloy_rpc_types_trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TraceResult};
use async_trait::async_trait;
use auto_impl::auto_impl;
use reth_primitives::{Block, BlockId, BlockNumberOrTag, Header, Log, Receipt, ReceiptWithBloom, TransactionSigned};
use std::sync::Arc;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait DebugProvider {
    async fn raw_header(&self, block_id: BlockId) -> EthApiResult<Bytes>;
    async fn raw_block(&self, block_id: BlockId) -> EthApiResult<Bytes>;
    async fn raw_transaction(&self, hash: B256) -> EthApiResult<Option<Bytes>>;
    async fn raw_transactions(&self, block_id: BlockId) -> EthApiResult<Vec<Bytes>>;
    async fn raw_receipts(&self, block_id: BlockId) -> EthApiResult<Vec<Bytes>>;
    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<Vec<TraceResult>>;
    async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<Vec<TraceResult>>;
    async fn trace_transaction(
        &self,
        transaction_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<GethTrace>;
    async fn trace_call(
        &self,
        request: TransactionRequest,
        block_number: Option<BlockId>,
        opts: Option<GethDebugTracingCallOptions>,
    ) -> EthApiResult<GethTrace>;
}

#[derive(Debug, Clone)]
pub struct DebugDataProvider<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> DebugDataProvider<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> DebugProvider for DebugDataProvider<P> {
    async fn raw_header(&self, block_id: BlockId) -> EthApiResult<Bytes> {
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

    async fn raw_block(&self, block_id: BlockId) -> EthApiResult<Bytes> {
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

    async fn raw_transaction(&self, hash: B256) -> EthApiResult<Option<Bytes>> {
        let transaction = self.eth_provider.transaction_by_hash(hash).await?;

        if let Some(tx) = transaction {
            let signature = tx.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = tx.try_into().map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::Primitive))?;
            let bytes = TransactionSigned::from_transaction_and_signature(
                tx,
                reth_primitives::Signature::from_rs_and_parity(
                    signature.r,
                    signature.s,
                    signature.y_parity.map_or(false, |v| v.0),
                )
                .expect("Invalid signature"),
            )
            .encoded_2718()
            .into();
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    async fn raw_transactions(&self, block_id: BlockId) -> EthApiResult<Vec<Bytes>> {
        let transactions = self.eth_provider.block_transactions(Some(block_id)).await?.unwrap_or_default();
        let mut raw_transactions = Vec::with_capacity(transactions.len());

        for t in transactions {
            let signature = t.signature.ok_or_else(|| EthApiError::from(SignatureError::MissingSignature))?;
            let tx = t.try_into().map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::Primitive))?;
            let bytes = TransactionSigned::from_transaction_and_signature(
                tx,
                reth_primitives::Signature::from_rs_and_parity(
                    signature.r,
                    signature.s,
                    signature.y_parity.map_or(false, |v| v.0),
                )
                .expect("Invalid signature"),
            )
            .encoded_2718()
            .into();
            raw_transactions.push(bytes);
        }

        Ok(raw_transactions)
    }

    async fn raw_receipts(&self, block_id: BlockId) -> EthApiResult<Vec<Bytes>> {
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
            let cumulative_gas_used = TryInto::<u64>::try_into(receipt.inner.inner.cumulative_gas_used())
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
                            .inner
                            .logs()
                            .iter()
                            .filter_map(|log| Log::new(log.address(), log.topics().to_vec(), log.data().data.clone()))
                            .collect(),
                    },
                    bloom: *receipt.inner.inner.logs_bloom(),
                }
                .envelope_encoded(),
            );
        }

        // Returns the vector containing the raw receipts
        Ok(raw_receipts)
    }

    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<Vec<TraceResult>> {
        let provider = Arc::new(&self.eth_provider);
        let tracer = TracerBuilder::new(provider)
            .await?
            .with_block_id(BlockId::Number(block_number))
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_block()?)
    }

    async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<Vec<TraceResult>> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(BlockId::Hash(block_hash.into()))
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_block()?)
    }

    async fn trace_transaction(
        &self,
        transaction_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthApiResult<GethTrace> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_transaction_hash(transaction_hash)
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_transaction(transaction_hash)?)
    }

    async fn trace_call(
        &self,
        request: TransactionRequest,
        block_number: Option<BlockId>,
        opts: Option<GethDebugTracingCallOptions>,
    ) -> EthApiResult<GethTrace> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(block_number.unwrap_or_default())
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_transaction_request(&request)?)
    }
}
