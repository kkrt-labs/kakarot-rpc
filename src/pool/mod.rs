use crate::providers::eth_provider::database::{
    ethereum::EthereumTransactionStore,
    filter::{self, EthDatabaseFilterBuilder},
    types::{oz_account::StoredOzAccount, transaction::StoredPendingTransaction},
    Database,
};
use eyre::Result;
use opentelemetry::metrics::{Gauge, Unit};
use reth_primitives::B256;
use std::{
    fmt,
    str::FromStr,
    time::{Duration, Instant},
};
use tokio::runtime::Handle;
use tracing::{instrument, Instrument};

pub mod mempool;
pub mod validate;

pub fn transaction_poll_interval() -> u64 {
    u64::from_str(
        &std::env::var("PROCESS_PENDING_TX_INTERVAL")
            .expect("Missing environment variable PROCESS_PENDING_TX_INTERVAL"),
    )
    .expect("failing to parse PROCESS_PENDING_TX_INTERVAL")
}

/// The [`PendingTxsHandler`] is responsible for processing pending transactions and reset the `OpenZeppelin` account fleet.
pub struct PendingTxsHandler {
    /// The database.
    database: Database,
    /// The time to process pending transactions.
    retry_time: Gauge<u64>,
}

impl fmt::Debug for PendingTxsHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PendingTxsHandler")
            .field("eth_provider", &"...")
            .field("database", &self.database)
            .finish_non_exhaustive()
    }
}

impl PendingTxsHandler {
    /// Creates a new [`PendingTxsHandler`] with the given Ethereum provider, database.
    pub fn new(database: Database) -> Self {
        let retry_time = opentelemetry::global::meter("retry_service")
            .u64_gauge("retry_time")
            .with_description("The time taken to process pending transactions")
            .with_unit(Unit::new("microseconds"))
            .init();
        Self { database, retry_time }
    }

    /// Spawns a new task on the provided tokio runtime that will pool transactions.
    #[instrument(skip_all, name = "pool_service")]
    pub fn start(self, rt_handle: &Handle) {
        tracing::info!("starting pool service");
        rt_handle.spawn(async move {
            loop {
                let start = Instant::now();
                if let Err(err) = self.process_pending_transactions().await {
                    tracing::error!(?err);
                }
                let end = Instant::now();
                let elapsed = end - start;
                self.retry_time.record(elapsed.as_micros() as u64, &[]);

                tokio::time::sleep(Duration::from_secs(transaction_poll_interval())).await;
            }
        });
    }

    /// Processes all current pending transactions by pruning them if necessary.
    ///
    /// It also resets the `OpenZeppelin` account fleet.
    async fn process_pending_transactions(&self) -> Result<()> {
        for transaction in self.pending_transactions().await? {
            if self.should_prune(&transaction).await? {
                self.prune_transaction(transaction.hash).await?;
            }
        }
        Ok(())
    }

    /// Prunes a transaction from the database given its hash.
    ///
    /// It also updates the associated `OpenZeppelin` account.
    async fn prune_transaction(&self, hash: B256) -> Result<()> {
        tracing::info!(%hash, "pruning");
        // Construct a filter to delete the transaction from the database.
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&hash).build();
        // Delete the transaction from the database.
        self.database.delete_one::<StoredPendingTransaction>(filter).await?;

        // Construct a filter to update the OZ account in the database.
        let account_filter = EthDatabaseFilterBuilder::<filter::OzAccount>::default().with_tx_hash(&hash).build();
        // Get the OZ account from the database.
        let oz_account =
            self.database.get_one::<StoredOzAccount>(account_filter.clone(), None).await?.unwrap_or_default();
        // Remove the current transaction hash from the OZ account.
        //
        // The OZ account is now free to accept a new transaction.
        self.database.update_one(StoredOzAccount { current_tx_hash: None, ..oz_account }, account_filter, true).await?;

        Ok(())
    }

    /// Returns all pending transactions.
    async fn pending_transactions(&self) -> Result<Vec<StoredPendingTransaction>> {
        let span = tracing::span!(tracing::Level::INFO, "db::pending_transactions");
        Ok(self.database.get_all().instrument(span).await?)
    }

    /// Returns true if the transaction should be pruned.
    ///
    /// A transaction should be pruned if it has been executed.
    async fn should_prune(&self, transaction: &StoredPendingTransaction) -> Result<bool> {
        Ok(self.database.transaction(&transaction.hash).await?.is_some())
    }
}
