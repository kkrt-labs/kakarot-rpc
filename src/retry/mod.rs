#![allow(clippy::used_underscore_binding)]
use crate::eth_provider::{
    database::{
        ethereum::EthereumTransactionStore,
        filter::{self, EthDatabaseFilterBuilder},
        types::transaction::StoredPendingTransaction,
        Database,
    },
    provider::EthereumProvider,
};
use eyre::Result;
use reth_primitives::{TransactionSignedEcRecovered, B256};
use std::{
    fmt,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
#[cfg(test)]
use {futures::lock::Mutex, std::sync::Arc};

pub fn get_retry_tx_interval() -> u64 {
    u64::from_str(&std::env::var("RETRY_TX_INTERVAL").expect("Missing environment variable RETRY_TX_INTERVAL"))
        .expect("failing to parse RETRY_TX_INTERVAL")
}

pub fn get_transaction_max_retries() -> u8 {
    u8::from_str(
        &std::env::var("TRANSACTION_MAX_RETRIES").expect("Missing environment variable TRANSACTION_MAX_RETRIES"),
    )
    .expect("failing to parse TRANSACTION_MAX_RETRIES")
}

/// The [`RetryHandler`] is responsible for retrying transactions that have failed.
pub struct RetryHandler<P: EthereumProvider> {
    /// The Ethereum provider.
    eth_provider: P,
    /// The database.
    database: Database,
    /// The retried transactions hashes.
    #[cfg(test)]
    retried: Arc<Mutex<Vec<B256>>>,
}

impl<P: EthereumProvider> fmt::Debug for RetryHandler<P> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryHandler")
            .field("eth_provider", &"...")
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl<P> RetryHandler<P>
where
    P: EthereumProvider + Send + Sync + 'static,
{
    /// Creates a new [`RetryHandler`] with the given Ethereum provider, database.
    #[allow(clippy::missing_const_for_fn)]
    pub fn new(eth_provider: P, database: Database) -> Self {
        Self {
            eth_provider,
            database,
            #[cfg(test)]
            retried: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Spawns a new task on the provided tokio runtime that will retry transactions.
    pub fn start(self, rt_handle: &Handle) {
        tracing::info!("Starting retry service");
        rt_handle.spawn(async move {
            let mut last_print_time = Instant::now();

            loop {
                let start_time_fn = Instant::now();
                if let Err(err) = self.process_pending_transactions().await {
                    tracing::error!("Error while retrying transactions: {:?}", err);
                }
                let elapsed_time_ms = start_time_fn.elapsed().as_millis();

                if last_print_time.elapsed() >= Duration::from_secs(300) {
                    tracing::info!("Elapsed time to retry transactions (milliseconds): {}", elapsed_time_ms);
                    last_print_time = Instant::now();
                }

                tokio::time::sleep(Duration::from_secs(get_retry_tx_interval())).await;
            }
        });
    }

    /// Processes all current pending transactions by retrying them
    /// and pruning them if necessary.
    async fn process_pending_transactions(&self) -> Result<()> {
        let pending_transactions = self.pending_transactions().await?;
        for transaction in pending_transactions {
            if self.should_retry(&transaction).await? {
                self.retry_transaction(transaction).await?;
            } else {
                self.prune_transaction(transaction.hash).await?;
            }
        }
        Ok(())
    }

    /// Retries a transaction and prunes it if the conversion to a primitive transaction fails.
    async fn retry_transaction(&self, transaction: StoredPendingTransaction) -> Result<()> {
        let hash = transaction.hash;
        tracing::info!("Retrying transaction {hash} with {} retries", transaction.retries + 1);

        // Generate primitive transaction, handle error if any
        let transaction = match TransactionSignedEcRecovered::try_from(transaction.tx.clone()) {
            Ok(transaction) => transaction,
            Err(error) => {
                self.prune_transaction(hash).await?;
                return Err(error.into());
            }
        };

        let _hash = self.eth_provider.send_raw_transaction(transaction.into_signed().envelope_encoded()).await?;
        #[cfg(test)]
        self.retried.lock().await.push(_hash);
        Ok(())
    }

    /// Prunes a transaction from the database given its hash.
    async fn prune_transaction(&self, hash: B256) -> Result<()> {
        tracing::info!("Pruning pending transaction: {hash}");
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&hash).build();
        self.database.delete_one::<StoredPendingTransaction>(filter).await?;
        Ok(())
    }

    /// Returns all pending transactions.
    async fn pending_transactions(&self) -> Result<Vec<StoredPendingTransaction>> {
        Ok(self.database.get_all().await?)
    }

    /// Returns true if the transaction should be retried. A transaction should be retried if it has
    /// not been executed and the number of retries is less than the maximum number of retries.
    async fn should_retry(&self, transaction: &StoredPendingTransaction) -> Result<bool> {
        let max_retries_reached = transaction.retries + 1 >= get_transaction_max_retries();
        let transaction_executed = self.database.transaction(&transaction.hash).await?.is_some();
        Ok(!max_retries_reached && !transaction_executed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{
        fixtures::{katana, setup},
        katana::Katana,
    };
    use rstest::*;

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_retry_handler(#[future] katana: Katana, _setup: ()) {
        // Given
        let eth_provider = katana.eth_provider();
        let db = eth_provider.database().clone();
        let retry_handler = RetryHandler::new(eth_provider.clone(), db.clone());

        // Insert the first transaction into the pending transactions collection with 0 retry
        let transaction1 = katana.eoa().mock_transaction_with_nonce(0).await.expect("Failed to get mock transaction");
        db.upsert_pending_transaction(transaction1.clone(), 0)
            .await
            .expect("Failed to insert pending transaction in database");

        // Insert the transaction into the pending transactions collection with TRANSACTION_MAX_RETRIES + 1 retry
        // Shouldn't be retried as it has reached the maximum number of retries
        let transaction2 = katana.eoa().mock_transaction_with_nonce(1).await.expect("Failed to get mock transaction");
        db.upsert_pending_transaction(transaction2.clone(), get_transaction_max_retries() + 1)
            .await
            .expect("Failed to insert pending transaction in database");

        // Insert the transaction into both the mined transactions and pending transactions collections
        // shouldn't be retried as it has already been mined
        let transaction3 = katana.eoa().mock_transaction_with_nonce(2).await.expect("Failed to get mock transaction");
        db.upsert_pending_transaction(transaction3.clone(), 0)
            .await
            .expect("Failed to insert pending transaction in database");
        db.upsert_transaction(transaction3.clone()).await.expect("Failed to insert transaction in mined collection");

        let mut pending_tx_hashes: Vec<B256> = Vec::new();
        let mut last_retried_transactions_hashes_len = retry_handler.retried.lock().await.len();

        for i in 0..get_transaction_max_retries() + 2 {
            // Retry transactions.
            retry_handler.process_pending_transactions().await.expect("Failed to retry transactions");

            // Retrieve the retried transactions.
            let retried = retry_handler.retried.lock().await.clone();
            // Slice the retried transactions that were not retried in the previous iteration.
            let retried_transaction_hashes = retried[last_retried_transactions_hashes_len..].to_vec();
            // Update the last retried transactions length.
            last_retried_transactions_hashes_len = retried.len();

            // Assert that there is only one retried transaction before reaching retry limit.
            assert_eq!(retried_transaction_hashes.len(), usize::from(i + 1 < get_transaction_max_retries()));

            // Retrieve the pending transactions.
            let pending_transactions =
                db.get_all::<StoredPendingTransaction>().await.expect("Failed get pending transactions");

            if i + 1 < get_transaction_max_retries() {
                // Ensure that the spurious transactions are dropped from the pending transactions collection
                assert_eq!(pending_transactions.len(), 1);

                // Ensure that the retry is incremented for the first transaction
                assert_eq!(pending_transactions.first().unwrap().retries, i + 1);

                // Ensure that the transaction1 is still in the pending transactions collection
                assert_eq!(pending_transactions.first().unwrap().tx, transaction1);

                // Get the pending transaction hash
                let pending_tx_hash = retried_transaction_hashes.first().unwrap();

                // Ensure that the pending transaction hash is not already in the list
                // Transaction hashes should be unique
                assert!(!pending_tx_hashes.contains(pending_tx_hash));

                // Add the pending transaction hash to the list
                pending_tx_hashes.push(*pending_tx_hash);
            } else {
                assert_eq!(pending_transactions.len(), 0);
            }
        }
    }
}
