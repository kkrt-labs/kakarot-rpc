use crate::{
    eth_provider::{
        provider::EthereumProvider,
    },
};
use auto_impl::auto_impl;
use reth_primitives::{
    Address, BlockId, BlockNumberOrTag, Bytes, B256, U256, U64,
};
use reth_rpc_types::{
    serde_helpers::JsonStorageKey,
    state::StateOverride,
    txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus},
    BlockOverrides, FeeHistory, Filter, FilterChanges, Header, Index, RichBlock,
    SyncStatus, Transaction, TransactionReceipt, TransactionRequest,
};
use async_trait::async_trait;
use crate::eth_provider::provider::EthProviderResult;


#[async_trait]
#[auto_impl(Arc, &)]
pub trait PoolProvider {
    async fn txpool_status(&self) -> EthProviderResult<TxpoolStatus>;
    async fn txpool_inspect(&self) -> EthProviderResult<TxpoolInspect>;
    async fn txpool_content_from(&self, from: Address) -> EthProviderResult<TxpoolContentFrom>;
    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent>;
}

#[derive(Debug, Clone)]
pub struct PoolStruct<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> PoolStruct<P> {
    pub fn new(eth_provider: P) -> Self {
        PoolStruct { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> EthereumProvider for PoolStruct<P> {
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>> {
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

    async fn block_by_number(&self, number_or_tag: BlockNumberOrTag, full: bool) -> EthProviderResult<Option<RichBlock>> {
        self.eth_provider.block_by_number(number_or_tag, full).await
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        self.eth_provider.block_transaction_count_by_hash(hash).await
    }

    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<Option<U256>> {
        self.eth_provider.block_transaction_count_by_number(number_or_tag).await
    }

    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_hash(hash).await
    }

    async fn transaction_by_block_hash_and_index(&self, hash: B256, index: Index) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_block_hash_and_index(hash, index).await
    }

    async fn transaction_by_block_number_and_index(&self, number_or_tag: BlockNumberOrTag, index: Index) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        self.eth_provider.transaction_by_block_number_and_index(number_or_tag, index).await
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        self.eth_provider.transaction_receipt(hash).await
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        self.eth_provider.balance(address, block_id).await
    }

    async fn storage_at(&self, address: Address, index: JsonStorageKey, block_id: Option<BlockId>) -> EthProviderResult<B256> {
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

    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>, state_overrides: Option<StateOverride>, block_overrides: Option<Box<BlockOverrides>>) -> EthProviderResult<Bytes> {
        self.eth_provider.call(request, block_id, state_overrides, block_overrides).await
    }

    async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        self.eth_provider.estimate_gas(call, block_id).await
    }

    async fn fee_history(&self, block_count: U64, newest_block: BlockNumberOrTag, reward_percentiles: Option<Vec<f64>>) -> EthProviderResult<FeeHistory> {
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

    async fn block_transactions(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>> {
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
impl<P: EthereumProvider + Send + Sync + 'static> PoolProvider for PoolStruct<P> {
    async fn txpool_status(&self) -> EthProviderResult<TxpoolStatus> {
        let all = self.eth_provider.txpool_content().await?;
        Ok(TxpoolStatus { pending: all.pending.len() as u64, queued: all.queued.len() as u64 })
    }

    async fn txpool_inspect(&self) -> EthProviderResult<TxpoolInspect> {
        let mut inspect = TxpoolInspect::default();

        let transactions = self.eth_provider.txpool_transactions().await?;

        for transaction in transactions {
            inspect.pending.entry(transaction.from).or_default().insert(
                transaction.nonce.to_string(),
                TxpoolInspectSummary {
                    to: transaction.to,
                    value: transaction.value,
                    gas: transaction.gas,
                    gas_price: transaction.gas_price.unwrap_or_default(),
                },
            );
        }

        Ok(inspect)
    }

    async fn txpool_content_from(&self, from: Address) -> EthProviderResult<TxpoolContentFrom> {
        Ok(self.eth_provider.txpool_content().await?.remove_from(&from))
    }

    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent> {
        Ok(self.eth_provider.txpool_content().await?)
    }
}