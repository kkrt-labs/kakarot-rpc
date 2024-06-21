use async_trait::async_trait;
use reth_primitives::B256;
use reth_rpc_types::Transaction;

use crate::eth_provider::error::EthApiError;

use super::filter;
use super::{
    filter::EthDatabaseFilterBuilder,
    types::transaction::{StoredPendingTransaction, StoredTransaction},
    Database,
};

/// Trait for interacting with a database that stores Ethereum typed
/// data.
#[async_trait]
pub trait EthereumDatabase {
    /// Returns the transaction with the given hash.
    async fn transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError>;
    /// Returns the pending transaction with the given hash.
    async fn pending_transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError>;
    /// Returns the pending transaction's retries with the given hash.
    /// Returns 0 if the transaction is not found.
    async fn pending_transaction_retries(&self, hash: &B256) -> Result<u8, EthApiError>;
    /// Upserts the given transaction.
    async fn upsert_transaction(&self, transaction: Transaction) -> Result<(), EthApiError>;
    /// Upserts the given transaction as a pending transaction with the given number of retries.
    async fn upsert_pending_transaction(&self, transaction: Transaction, retries: u8) -> Result<(), EthApiError>;
}

#[async_trait]
impl EthereumDatabase for Database {
    async fn transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(hash).build();
        Ok(self.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn pending_transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(hash).build();
        Ok(self.get_one::<StoredPendingTransaction>(filter, None).await?.map(Into::into))
    }

    async fn pending_transaction_retries(&self, hash: &B256) -> Result<u8, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(hash).build();
        Ok(self
            .get_one::<StoredPendingTransaction>(filter, None)
            .await?
            .map(|tx| tx.retries + 1)
            .inspect(|retries| tracing::info!("Retrying {} with {} retries", hash, retries))
            .or_else(|| {
                tracing::info!("New transaction {} in pending pool", hash);
                None
            })
            .unwrap_or_default())
    }

    async fn upsert_transaction(&self, transaction: Transaction) -> Result<(), EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&transaction.hash).build();
        Ok(self.update_one(StoredTransaction::from(transaction), filter, true).await?)
    }

    async fn upsert_pending_transaction(&self, transaction: Transaction, retries: u8) -> Result<(), EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&transaction.hash).build();
        Ok(self.update_one(StoredPendingTransaction::new(transaction, retries), filter, true).await?)
    }
}
