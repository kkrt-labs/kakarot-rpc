use crate::{
    eth_provider::{
        error::{EthApiError, EthereumDataFormatError, SignatureError},
        provider::{EthProviderResult, EthereumProvider},
    },
    tracing::builder::TracerBuilder,
};
use alloy_rlp::Encodable;
use async_trait::async_trait;
use auto_impl::auto_impl;
use reth_primitives::{
    Address, Block, BlockId, BlockNumberOrTag, Bytes, Header, Log, Receipt, ReceiptWithBloom, TransactionSigned, B256,
    U256, U64,
};
use reth_rpc_types::{
    serde_helpers::JsonStorageKey,
    state::StateOverride,
    trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions, GethTrace, TraceResult},
    txpool::TxpoolContent,
    BlockOverrides, FeeHistory, Filter, FilterChanges, Header as hr, Index, RichBlock, SyncStatus, Transaction,
    TransactionReceipt, TransactionRequest,
};
use std::sync::Arc;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait DebugProvider {
    async fn raw_header(&self, block_id: BlockId) -> EthProviderResult<Bytes>;
    async fn raw_block(&self, block_id: BlockId) -> EthProviderResult<Bytes>;
    async fn raw_transaction(&self, hash: B256) -> EthProviderResult<Option<Bytes>>;
    async fn raw_transactions(&self, block_id: BlockId) -> EthProviderResult<Vec<Bytes>>;
    async fn raw_receipts(&self, block_id: BlockId) -> EthProviderResult<Vec<Bytes>>;
    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthProviderResult<Vec<TraceResult>>;
    async fn trace_block_by_hash(
        &self,
        block_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthProviderResult<Vec<TraceResult>>;
    async fn trace_transaction(
        &self,
        transaction_hash: B256,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthProviderResult<GethTrace>;
    async fn trace_call(
        &self,
        request: TransactionRequest,
        block_number: Option<BlockId>,
        opts: Option<GethDebugTracingCallOptions>,
    ) -> EthProviderResult<GethTrace>;
}

#[derive(Debug, Clone)]
pub struct DebugStruct<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> DebugStruct<P> {
    pub fn new(eth_provider: P) -> Self {
        DebugStruct { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> EthereumProvider for DebugStruct<P> {
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<hr>> {
        self.eth_provider.header(block_id).await
    }

    async fn block_number(&self) -> EthProviderResult<U64> {
        self.eth_provider.block_number().await
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        self.eth_provider.syncing().await
    }

    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        self.eth_provider.chain_id().await
    }

    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        self.eth_provider.block_by_hash(hash, full).await
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        self.eth_provider.block_by_number(number_or_tag, full).await
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        self.eth_provider.block_transaction_count_by_hash(hash).await
    }

    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>> {
        self.eth_provider.block_transaction_count_by_number(number_or_tag).await
    }

    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_hash(hash).await
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_block_hash_and_index(hash, index).await
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_block_number_and_index(number_or_tag, index).await
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        self.eth_provider.transaction_receipt(hash).await
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        self.eth_provider.balance(address, block_id).await
    }

    async fn storage_at(
        &self,
        address: Address,
        index: JsonStorageKey,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<B256> {
        self.eth_provider.storage_at(address, index, block_id).await
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        self.eth_provider.transaction_count(address, block_id).await
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        self.eth_provider.get_code(address, block_id).await
    }

    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        self.eth_provider.get_logs(filter).await
    }

    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> EthProviderResult<Bytes> {
        self.eth_provider.call(request, block_id, state_overrides, block_overrides).await
    }

    async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        self.eth_provider.estimate_gas(call, block_id).await
    }

    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory> {
        self.eth_provider.fee_history(block_count, newest_block, reward_percentiles).await
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256> {
        self.eth_provider.send_raw_transaction(transaction).await
    }

    async fn gas_price(&self) -> EthProviderResult<U256> {
        self.eth_provider.gas_price().await
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<TransactionReceipt>>> {
        self.eth_provider.block_receipts(block_id).await
    }

    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>> {
        self.eth_provider.block_transactions(block_id).await
    }

    async fn txpool_transactions(&self) -> EthProviderResult<Vec<Transaction>> {
        self.eth_provider.txpool_transactions().await
    }

    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent> {
        self.eth_provider.txpool_content().await
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> DebugProvider for DebugStruct<P> {
    async fn raw_header(&self, block_id: BlockId) -> EthProviderResult<Bytes> {
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

    async fn raw_block(&self, block_id: BlockId) -> EthProviderResult<Bytes> {
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

    async fn raw_transaction(&self, hash: B256) -> EthProviderResult<Option<Bytes>> {
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

    async fn raw_transactions(&self, block_id: BlockId) -> EthProviderResult<Vec<Bytes>> {
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

    async fn raw_receipts(&self, block_id: BlockId) -> EthProviderResult<Vec<Bytes>> {
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

    async fn trace_block_by_number(
        &self,
        block_number: BlockNumberOrTag,
        opts: Option<GethDebugTracingOptions>,
    ) -> EthProviderResult<Vec<TraceResult>> {
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
    ) -> EthProviderResult<Vec<TraceResult>> {
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
    ) -> EthProviderResult<GethTrace> {
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
    ) -> EthProviderResult<GethTrace> {
        let tracer = TracerBuilder::new(Arc::new(&self.eth_provider))
            .await?
            .with_block_id(block_number.unwrap_or_default())
            .await?
            .with_tracing_options(opts.unwrap_or_default().into())
            .build()?;

        Ok(tracer.debug_transaction_request(&request)?)
    }
}
