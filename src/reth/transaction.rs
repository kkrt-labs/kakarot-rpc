use super::{KakarotEthApi, KakarotProvider};
use reth_primitives::{
    Address, BlockHashOrNumber, BlockNumber, TransactionMeta, TransactionSigned, TransactionSignedNoHash, TxHash,
    TxNumber,
};
use reth_rpc_eth_api::{
    helpers::{EthSigner, EthTransactions, LoadTransaction, SpawnBlocking},
    RawTransactionForwarder,
};
use reth_rpc_eth_types::EthStateCache;
use reth_storage_api::{errors::provider::ProviderResult, BlockReaderIdExt, TransactionsProvider};
use reth_transaction_pool::TransactionPool;
use std::ops::RangeBounds;

impl TransactionsProvider for KakarotProvider {
    fn transaction_id(&self, _tx_hash: TxHash) -> ProviderResult<Option<TxNumber>> {
        Ok(None)
    }

    fn transaction_by_id(&self, _id: TxNumber) -> ProviderResult<Option<TransactionSigned>> {
        Ok(None)
    }

    fn transaction_by_id_no_hash(&self, _id: TxNumber) -> ProviderResult<Option<TransactionSignedNoHash>> {
        Ok(None)
    }

    fn transaction_by_hash(&self, _hash: TxHash) -> ProviderResult<Option<TransactionSigned>> {
        Ok(None)
    }

    fn transaction_by_hash_with_meta(
        &self,
        _hash: TxHash,
    ) -> ProviderResult<Option<(TransactionSigned, TransactionMeta)>> {
        Ok(None)
    }

    fn transaction_block(&self, _id: TxNumber) -> ProviderResult<Option<BlockNumber>> {
        Ok(None)
    }

    fn transactions_by_block(&self, _block: BlockHashOrNumber) -> ProviderResult<Option<Vec<TransactionSigned>>> {
        Ok(None)
    }

    fn transactions_by_block_range(
        &self,
        _range: impl RangeBounds<BlockNumber>,
    ) -> ProviderResult<Vec<Vec<TransactionSigned>>> {
        Ok(vec![])
    }

    fn transactions_by_tx_range(
        &self,
        _range: impl RangeBounds<TxNumber>,
    ) -> ProviderResult<Vec<TransactionSignedNoHash>> {
        Ok(vec![])
    }

    fn senders_by_tx_range(&self, _range: impl RangeBounds<TxNumber>) -> ProviderResult<Vec<Address>> {
        Ok(vec![])
    }

    fn transaction_sender(&self, _id: TxNumber) -> ProviderResult<Option<Address>> {
        Ok(None)
    }
}

impl<Provider, Pool, Network, EvmConfig> LoadTransaction for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: SpawnBlocking,
    Provider: TransactionsProvider,
    Pool: TransactionPool,
{
    type Pool = Pool;

    #[inline]
    fn provider(&self) -> impl TransactionsProvider {
        self.0.provider()
    }

    #[inline]
    fn cache(&self) -> &EthStateCache {
        self.0.cache()
    }

    #[inline]
    fn pool(&self) -> &Self::Pool {
        self.0.pool()
    }
}

impl<Provider, Pool, Network, EvmConfig> EthTransactions for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: LoadTransaction,
    Pool: Send + Sync + 'static,
    Provider: BlockReaderIdExt,
{
    #[inline]
    fn provider(&self) -> impl BlockReaderIdExt {
        self.0.provider()
    }

    #[inline]
    fn raw_tx_forwarder(&self) -> Option<std::sync::Arc<dyn RawTransactionForwarder>> {
        self.0.raw_tx_forwarder()
    }

    #[inline]
    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn EthSigner>>> {
        self.0.signers()
    }
}
