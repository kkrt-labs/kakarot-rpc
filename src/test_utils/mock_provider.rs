use crate::providers::eth_provider::{
    provider::EthApiResult, BlockProvider, ChainProvider, GasProvider, LogProvider, ReceiptProvider, StateProvider,
    TransactionProvider,
};
use async_trait::async_trait;
use mockall::mock;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, B256, U256, U64};
use reth_rpc_types::{Filter, FilterChanges, Header, SyncStatus, TransactionReceipt, TransactionRequest};

mock! {
    #[derive(Clone, Debug)]
    pub EthereumProviderStruct {}


    #[async_trait]
    impl BlockProvider for EthereumProviderStruct {
        async fn header(&self, block_id: &BlockId) -> EthApiResult<Option<Header>>;

        async fn block_number(&self) -> EthApiResult<U64>;

        async fn block_by_hash(&self, hash: B256, full: bool) -> EthApiResult<Option<reth_rpc_types::RichBlock>>;

        async fn block_by_number(&self, number_or_tag: BlockNumberOrTag, full: bool) -> EthApiResult<Option<reth_rpc_types::RichBlock>>;

        async fn block_transaction_count_by_hash(&self, hash: B256) -> EthApiResult<Option<U256>>;

        async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthApiResult<Option<U256>>;

        async fn block_transactions(&self, block_id: Option<BlockId>) -> EthApiResult<Option<Vec<reth_rpc_types::Transaction>>>;
    }

    #[async_trait]
    impl ChainProvider for EthereumProviderStruct {
        async fn syncing(&self) -> EthApiResult<SyncStatus>;

        async fn chain_id(&self) -> EthApiResult<Option<U64>>;
    }

    #[async_trait]
    impl GasProvider for EthereumProviderStruct {
        async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthApiResult<U256>;

        async fn fee_history(&self, block_count: U64, newest_block: BlockNumberOrTag, reward_percentiles: Option<Vec<f64>>) -> EthApiResult<reth_rpc_types::FeeHistory>;

        async fn gas_price(&self) -> EthApiResult<U256>;
    }

    #[async_trait]
    impl LogProvider for EthereumProviderStruct {
        async fn get_logs(&self, filter: Filter) -> EthApiResult<FilterChanges>;
    }

    #[async_trait]
    impl ReceiptProvider for EthereumProviderStruct {
        async fn transaction_receipt(&self, hash: B256) -> EthApiResult<Option<TransactionReceipt>>;

        async fn block_receipts(&self, block_id: Option<BlockId>) -> EthApiResult<Option<Vec<TransactionReceipt>>>;
    }

    #[async_trait]
    impl StateProvider for EthereumProviderStruct {
        async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256>;

        async fn storage_at(&self, address: Address, index: reth_rpc_types::serde_helpers::JsonStorageKey, block_id: Option<BlockId>) -> EthApiResult<B256>;

        async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<Bytes>;

        async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>, state_overrides: Option<reth_rpc_types::state::StateOverride>, block_overrides: Option<Box<reth_rpc_types::BlockOverrides>>) -> EthApiResult<Bytes>;
    }

    #[async_trait]
    impl TransactionProvider for EthereumProviderStruct {
        async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

        async fn transaction_by_block_hash_and_index(&self, hash: B256, index: reth_rpc_types::Index) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

        async fn transaction_by_block_number_and_index(&self, number_or_tag: BlockNumberOrTag, index: reth_rpc_types::Index) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

        async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256>;
    }

    // #[async_trait]
    // impl TxPoolProvider for EthereumProviderStruct {
    //     async fn txpool_transactions(&self) -> EthApiResult<Vec<reth_rpc_types::Transaction>>;

    //     async fn txpool_content(&self) -> EthApiResult<TxpoolContent>;
    // }
}
