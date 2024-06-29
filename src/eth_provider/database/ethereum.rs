use alloy_rlp::Encodable;
use async_trait::async_trait;
use mongodb::bson::doc;
use reth_primitives::constants::EMPTY_ROOT_HASH;
use reth_primitives::{TransactionSigned, B256, U256};
use reth_rpc_types::{Block, BlockHashOrNumber, BlockTransactions, Header, RichBlock, Transaction};

use crate::eth_provider::error::{EthApiError, EthereumDataFormatError};

use super::filter;
use super::types::header::StoredHeader;
use super::{
    filter::EthDatabaseFilterBuilder,
    types::transaction::{StoredPendingTransaction, StoredTransaction},
    Database,
};

/// Trait for interacting with a database that stores Ethereum typed
/// transaction data.
#[async_trait]
pub trait EthereumTransactionStore {
    /// Returns the transaction with the given hash. Returns None if the
    /// transaction is not found.
    async fn transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError>;
    /// Returns all transactions for the given block hash or number.
    async fn transactions(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Vec<Transaction>, EthApiError>;
    /// Returns all transactions hashes for the given block hash or number.
    async fn transaction_hashes(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Vec<B256>, EthApiError>;
    /// Returns the pending transaction with the given hash. Returns None if the
    /// transaction is not found.
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
impl EthereumTransactionStore for Database {
    async fn transaction(&self, hash: &B256) -> Result<Option<Transaction>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(hash).build();
        Ok(self.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transactions(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Vec<Transaction>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();

        Ok(self.get::<StoredTransaction>(filter, None).await?.into_iter().map(Into::into).collect())
    }

    async fn transaction_hashes(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Vec<B256>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();

        Ok(self.get::<StoredTransaction>(filter, None).await?.into_iter().map(|tx| tx.tx.hash).collect())
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

/// Trait for interacting with a database that stores Ethereum typed
/// blocks.
#[async_trait]
pub trait EthereumBlockStore {
    /// Returns the header for the given hash or number. Returns None if the
    /// header is not found.
    async fn header(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError>;
    /// Returns the block for the given hash or number. Returns None if the
    /// block is not found.
    async fn block(
        &self,
        block_hash_or_number: BlockHashOrNumber,
        full: bool,
    ) -> Result<Option<RichBlock>, EthApiError>;
    /// Returns true if the block with the given hash or number exists.
    async fn block_exists(&self, block_hash_or_number: BlockHashOrNumber) -> Result<bool, EthApiError> {
        self.header(block_hash_or_number).await.map(|header| header.is_some())
    }
    /// Returns the transaction count for the given block hash or number. Returns None if the
    /// block is not found.
    async fn transaction_count(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<U256>, EthApiError>;
}

#[async_trait]
impl EthereumBlockStore for Database {
    async fn header(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Header>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();
        Ok(self
            .get_one::<StoredHeader>(filter, None)
            .await
            .inspect_err(|err| tracing::error!("internal error: {:?}", err))
            .map_err(|_| EthApiError::UnknownBlock(block_hash_or_number))?
            .map(|sh| sh.header))
    }

    async fn block(
        &self,
        block_hash_or_number: BlockHashOrNumber,
        full: bool,
    ) -> Result<Option<RichBlock>, EthApiError> {
        let maybe_header = self.header(block_hash_or_number).await?;
        if maybe_header.is_none() {
            return Ok(None);
        }
        let header = maybe_header.unwrap();

        // The withdrawals are not supported, hence the withdrawals_root should always be empty.
        if let Some(withdrawals_root) = header.withdrawals_root {
            if withdrawals_root != EMPTY_ROOT_HASH {
                return Err(EthApiError::Unsupported("withdrawals"));
            }
        }

        let transactions = self.transactions(block_hash_or_number).await?;
        let block_transactions = if full {
            BlockTransactions::Full(transactions.clone())
        } else {
            BlockTransactions::Hashes(transactions.iter().map(|tx| tx.hash).collect())
        };

        let signed_transactions = transactions
            .into_iter()
            .map(|tx| TransactionSigned::try_from(tx).map_err(|_| EthereumDataFormatError::TransactionConversion))
            .collect::<Result<Vec<_>, _>>()?;

        let block = reth_primitives::Block {
            body: signed_transactions,
            header: reth_primitives::Header::try_from(header.clone())
                .map_err(|_| EthereumDataFormatError::Primitive)?,
            withdrawals: Some(Default::default()),
            ..Default::default()
        };

        // This is how Reth computes the block size.
        // `https://github.com/paradigmxyz/reth/blob/v0.2.0-beta.5/crates/rpc/rpc-types-compat/src/block.rs#L66`
        let size = block.length();

        Ok(Some(
            Block {
                header,
                transactions: block_transactions,
                size: Some(U256::from(size)),
                withdrawals: Some(Default::default()),
                ..Default::default()
            }
            .into(),
        ))
    }

    async fn transaction_count(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<U256>, EthApiError> {
        if !self.block_exists(block_hash_or_number).await? {
            return Ok(None);
        }

        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();
        let count = self.count::<StoredTransaction>(filter).await?;
        Ok(Some(U256::from(count)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mongo::{CollectionDB, MongoFuzzer, DOCKER_CLI, RANDOM_BYTES_SIZE};

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ethereum_transaction_store() {
        // Initialize MongoDB fuzzer
        let mut mongo_fuzzer = MongoFuzzer::new(RANDOM_BYTES_SIZE).await;

        // Start MongoDB Docker container
        let _c = DOCKER_CLI.run(mongo_fuzzer.mongo_image());

        // Mock a database with 100 transactions, receipts, and headers
        let database = mongo_fuzzer.mock_database(100).await;

        // Generate random bytes for test data
        let bytes: Vec<u8> = (0..RANDOM_BYTES_SIZE).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);

        // Test fetching existing and non existing transactions by their hash
        test_get_transaction(&database, &mongo_fuzzer, &mut unstructured).await;

        // Test fetching transactions by their block hash
        test_get_transactions_by_block_hash(&database, &mongo_fuzzer).await;

        // Test fetching transactions by their block number
        test_get_transactions_by_block_number(&database, &mongo_fuzzer).await;

        // Test upserting pending transactions into the database
        test_upsert_pending_transactions(&mut unstructured, &database).await;

        // Test upserting transactions into the database
        test_upsert_transactions(&mut unstructured, &database).await;
    }

    async fn test_get_transaction(
        database: &Database,
        mongo_fuzzer: &MongoFuzzer,
        unstructured: &mut arbitrary::Unstructured<'_>,
    ) {
        // Fetch the first transaction from the mock database
        let first_transaction = mongo_fuzzer
            .documents()
            .get(&CollectionDB::Transactions)
            .unwrap()
            .first()
            .unwrap()
            .extract_stored_transaction()
            .unwrap();

        // Test retrieving an existing transaction by its hash
        assert_eq!(database.transaction(&first_transaction.tx.hash).await.unwrap(), Some(first_transaction.tx.clone()));

        // Generate a transaction not present in the database
        let unstored_transaction = StoredTransaction::arbitrary_with_optional_fields(unstructured).unwrap().tx;

        // Test retrieving a non-existent transaction by its hash
        assert_eq!(database.transaction(&unstored_transaction.hash).await.unwrap(), None);
    }

    async fn test_get_transactions_by_block_hash(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        // Fetch the first block hash from the mock database
        let first_block_hash = mongo_fuzzer
            .documents()
            .get(&CollectionDB::Headers)
            .unwrap()
            .first()
            .unwrap()
            .extract_stored_header()
            .unwrap()
            .header
            .hash
            .unwrap();

        // Fetch transactions belonging to the first block hash
        let transactions_first_block_hash = mongo_fuzzer
            .documents()
            .get(&CollectionDB::Transactions)
            .unwrap()
            .iter()
            .filter(|tx| tx.extract_stored_transaction().unwrap().tx.block_hash.unwrap() == first_block_hash)
            .map(|stored_data| stored_data.extract_stored_transaction().unwrap().tx.clone())
            .collect::<Vec<_>>();

        // Test retrieving transactions by block hash
        assert_eq!(database.transactions(first_block_hash.into()).await.unwrap(), transactions_first_block_hash);

        // Test retrieving transaction hashes by block hash
        assert_eq!(
            database.transaction_hashes(first_block_hash.into()).await.unwrap(),
            transactions_first_block_hash.iter().map(|tx| tx.hash).collect::<Vec<_>>()
        );
    }

    async fn test_get_transactions_by_block_number(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        // Fetch the first block number from the mock database
        let first_block_number = mongo_fuzzer
            .documents()
            .get(&CollectionDB::Headers)
            .unwrap()
            .first()
            .unwrap()
            .extract_stored_header()
            .unwrap()
            .header
            .number
            .unwrap();

        // Fetch transactions belonging to the first block number
        let transactions_first_block_number = mongo_fuzzer
            .documents()
            .get(&CollectionDB::Transactions)
            .unwrap()
            .iter()
            .filter(|tx| tx.extract_stored_transaction().unwrap().tx.block_number.unwrap() == first_block_number)
            .map(|stored_data| stored_data.extract_stored_transaction().unwrap().tx.clone())
            .collect::<Vec<_>>();

        // Test retrieving transactions by block number
        assert_eq!(database.transactions(first_block_number.into()).await.unwrap(), transactions_first_block_number);

        // Test retrieving transaction hashes by block number
        assert_eq!(
            database.transaction_hashes(first_block_number.into()).await.unwrap(),
            transactions_first_block_number.iter().map(|tx| tx.hash).collect::<Vec<_>>()
        );
    }

    async fn test_upsert_pending_transactions(unstructured: &mut arbitrary::Unstructured<'_>, database: &Database) {
        // Generate 10 pending transactions and add them to the database
        let pending_transactions: Vec<StoredPendingTransaction> =
            (0..10).map(|_| StoredPendingTransaction::arbitrary_with_optional_fields(unstructured).unwrap()).collect();

        // Add pending transactions to the database
        for tx in &pending_transactions {
            database
                .upsert_pending_transaction(tx.tx.clone(), tx.retries)
                .await
                .expect("Failed to update pending transaction in database");
        }

        // Test retrieving a pending transaction by its hash
        let first_pending_transaction = pending_transactions.first().unwrap();
        assert_eq!(
            database.pending_transaction(&first_pending_transaction.tx.hash).await.unwrap(),
            Some(first_pending_transaction.tx.clone())
        );

        // Test retrieving a non-existent pending transaction by its hash
        let unstored_transaction = StoredTransaction::arbitrary_with_optional_fields(unstructured).unwrap().tx;
        assert_eq!(database.pending_transaction(&unstored_transaction.hash).await.unwrap(), None);

        // Test retrieving the number of retries for a pending transaction
        assert_eq!(
            database.pending_transaction_retries(&first_pending_transaction.tx.hash).await.unwrap(),
            first_pending_transaction.clone().retries.saturating_add(1)
        );

        // Test retrieving the number of retries for a non-existent pending transaction
        assert_eq!(database.pending_transaction_retries(&unstored_transaction.hash).await.unwrap(), 0);
    }

    async fn test_upsert_transactions(unstructured: &mut arbitrary::Unstructured<'_>, database: &Database) {
        // Generate and upsert a mock transaction into the database
        let mock_transaction = StoredTransaction::arbitrary_with_optional_fields(unstructured).unwrap();
        database.upsert_transaction(mock_transaction.clone().tx).await.unwrap();

        // Test retrieving an upserted transaction by its hash
        assert_eq!(database.transaction(&mock_transaction.tx.hash).await.unwrap(), Some(mock_transaction.tx.clone()));

        // Generate and upsert a mock pending transaction into the database
        let mock_pending_transaction = StoredPendingTransaction::arbitrary_with_optional_fields(unstructured).unwrap();
        database
            .upsert_pending_transaction(mock_pending_transaction.clone().tx, mock_pending_transaction.clone().retries)
            .await
            .unwrap();

        // Test retrieving an upserted pending transaction by its hash
        assert_eq!(
            database.pending_transaction(&mock_pending_transaction.clone().tx.hash).await.unwrap(),
            Some(mock_pending_transaction.clone().tx)
        );
    }
}
