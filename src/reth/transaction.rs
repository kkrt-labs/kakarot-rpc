use super::KakarotProvider;
use reth_primitives::{
    Address, BlockHashOrNumber, BlockNumber, TransactionMeta, TransactionSigned, TransactionSignedNoHash, TxHash,
    TxNumber,
};
use reth_storage_api::{errors::provider::ProviderResult, TransactionsProvider};
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
