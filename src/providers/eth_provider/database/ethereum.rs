#![allow(clippy::blocks_in_conditions)]
use super::{
    filter,
    filter::EthDatabaseFilterBuilder,
    types::{header::StoredHeader, transaction::StoredTransaction},
    Database,
};
use crate::providers::eth_provider::error::{EthApiError, EthereumDataFormatError};
use alloy_primitives::{B256, U256};
use alloy_rlp::Encodable;
use alloy_rpc_types::{Block, BlockHashOrNumber, BlockTransactions, Header, Transaction};
use alloy_serde::WithOtherFields;
use async_trait::async_trait;
use mongodb::bson::doc;
use reth_primitives::{constants::EMPTY_ROOT_HASH, TransactionSigned};
use tracing::instrument;

/// Trait for interacting with a database that stores Ethereum typed
/// transaction data.
#[async_trait]
pub trait EthereumTransactionStore {
    /// Returns the transaction with the given hash. Returns None if the
    /// transaction is not found.
    async fn transaction(&self, hash: &B256) -> Result<Option<WithOtherFields<Transaction>>, EthApiError>;
    /// Returns all transactions for the given block hash or number.
    async fn transactions(
        &self,
        block_hash_or_number: BlockHashOrNumber,
    ) -> Result<Vec<WithOtherFields<Transaction>>, EthApiError>;
    /// Upserts the given transaction.
    async fn upsert_transaction(&self, transaction: WithOtherFields<Transaction>) -> Result<(), EthApiError>;
}

#[async_trait]
impl EthereumTransactionStore for Database {
    #[instrument(skip_all, name = "db::transaction", err)]
    async fn transaction(&self, hash: &B256) -> Result<Option<WithOtherFields<Transaction>>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(hash).build();
        Ok(self.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    #[instrument(skip_all, name = "db::transactions", err)]
    async fn transactions(
        &self,
        block_hash_or_number: BlockHashOrNumber,
    ) -> Result<Vec<WithOtherFields<Transaction>>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();

        Ok(self.get::<StoredTransaction>(filter, None).await?.into_iter().map(Into::into).collect())
    }

    #[instrument(skip_all, name = "db::upsert_transaction", err)]
    async fn upsert_transaction(&self, transaction: WithOtherFields<Transaction>) -> Result<(), EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&transaction.hash).build();
        Ok(self.update_one(StoredTransaction::from(transaction), filter, true).await?)
    }
}

/// Trait for interacting with a database that stores Ethereum typed
/// blocks.
#[async_trait]
pub trait EthereumBlockStore {
    /// Returns the latest block header.
    async fn latest_header(&self) -> Result<Option<Header>, EthApiError>;
    /// Returns the header for the given hash or number. Returns None if the
    /// header is not found.
    async fn header(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError>;
    /// Returns the block for the given hash or number. Returns None if the
    /// block is not found.
    async fn block(
        &self,
        block_hash_or_number: BlockHashOrNumber,
        full: bool,
    ) -> Result<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>, EthApiError>;
    /// Returns true if the block with the given hash or number exists.
    #[instrument(skip(self), name = "db::block_exists", err)]
    async fn block_exists(&self, block_hash_or_number: BlockHashOrNumber) -> Result<bool, EthApiError> {
        self.header(block_hash_or_number).await.map(|header| header.is_some())
    }
    /// Returns the transaction count for the given block hash or number. Returns None if the
    /// block is not found.
    async fn transaction_count(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<U256>, EthApiError>;
}

#[async_trait]
impl EthereumBlockStore for Database {
    #[instrument(skip_all, name = "db::latest_header", err)]
    async fn latest_header(&self) -> Result<Option<Header>, EthApiError> {
        Ok(self
            .get_one::<StoredHeader>(None, doc! { "header.number": -1 })
            .await
            .map(|maybe_sh| maybe_sh.map(Into::into))?)
    }

    #[instrument(skip_all, name = "db::header", err)]
    async fn header(&self, block_hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Header>::default()
            .with_block_hash_or_number(block_hash_or_number)
            .build();
        Ok(self
            .get_one::<StoredHeader>(filter, None)
            .await
            .map_err(|_| EthApiError::UnknownBlock(block_hash_or_number))?
            .map(Into::into))
    }

    #[instrument(skip_all, name = "db::block", err)]
    async fn block(
        &self,
        block_hash_or_number: BlockHashOrNumber,
        full: bool,
    ) -> Result<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>, EthApiError> {
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
            body: reth_primitives::BlockBody { transactions: signed_transactions, ..Default::default() },
            header: reth_primitives::Header::try_from(header.clone())
                .map_err(|_| EthereumDataFormatError::Primitive)?,
        };

        // This is how Reth computes the block size.
        // `https://github.com/paradigmxyz/reth/blob/v0.2.0-beta.5/crates/rpc/rpc-types-compat/src/block.rs#L66`
        let size = block.length();

        Ok(Some(WithOtherFields::new(Block {
            header,
            transactions: block_transactions,
            size: Some(U256::from(size)),
            withdrawals: Some(Default::default()),
            ..Default::default()
        })))
    }

    #[instrument(skip_all, name = "db::transaction_count", err)]
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
    use crate::test_utils::mongo::{MongoFuzzer, RANDOM_BYTES_SIZE};
    use arbitrary::Arbitrary;
    use rand::{self, Rng};

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ethereum_transaction_store() {
        // Initialize MongoDB fuzzer
        let mut mongo_fuzzer = MongoFuzzer::new(RANDOM_BYTES_SIZE).await;

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

        // Test upserting transactions into the database
        test_upsert_transactions(&mut unstructured, &database).await;
    }

    async fn test_get_transaction(
        database: &Database,
        mongo_fuzzer: &MongoFuzzer,
        unstructured: &mut arbitrary::Unstructured<'_>,
    ) {
        // Fetch the first transaction from the mock database
        let first_transaction = mongo_fuzzer.transactions.first().unwrap();

        // Test retrieving an existing transaction by its hash
        assert_eq!(database.transaction(&first_transaction.hash).await.unwrap(), Some(first_transaction.into()));

        // Generate a transaction not present in the database
        let unstored_transaction = StoredTransaction::arbitrary(unstructured).unwrap();

        // Test retrieving a non-existent transaction by its hash
        assert_eq!(database.transaction(&unstored_transaction.hash).await.unwrap(), None);
    }

    async fn test_get_transactions_by_block_hash(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        // Fetch the first block hash from the mock database
        let first_block_hash = mongo_fuzzer.headers.first().unwrap().hash;

        // Fetch transactions belonging to the first block hash
        let transactions_first_block_hash = mongo_fuzzer
            .transactions
            .iter()
            .filter(|tx| tx.block_hash.unwrap() == first_block_hash)
            .map(Into::into)
            .collect::<Vec<_>>();

        // Test retrieving transactions by block hash
        assert_eq!(database.transactions(first_block_hash.into()).await.unwrap(), transactions_first_block_hash);
    }

    async fn test_get_transactions_by_block_number(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        // Fetch the first block number from the mock database
        let first_block_number = mongo_fuzzer.headers.first().unwrap().number;

        // Fetch transactions belonging to the first block number
        let transactions_first_block_number = mongo_fuzzer
            .transactions
            .iter()
            .filter(|tx| tx.block_number.unwrap() == first_block_number)
            .map(Into::into)
            .collect::<Vec<_>>();

        // Test retrieving transactions by block number
        assert_eq!(database.transactions(first_block_number.into()).await.unwrap(), transactions_first_block_number);
    }

    async fn test_upsert_transactions(unstructured: &mut arbitrary::Unstructured<'_>, database: &Database) {
        // Generate and upsert a mock transaction into the database
        let mock_transaction = StoredTransaction::arbitrary(unstructured).unwrap();
        database.upsert_transaction(mock_transaction.clone().tx).await.unwrap();

        // Test retrieving an upserted transaction by its hash
        assert_eq!(database.transaction(&mock_transaction.hash).await.unwrap(), Some(mock_transaction.into()));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_ethereum_block_store() {
        // Initialize MongoDB fuzzer
        let mut mongo_fuzzer = MongoFuzzer::new(RANDOM_BYTES_SIZE).await;

        // Mock a database with 100 transactions, receipts, and headers
        let database = mongo_fuzzer.mock_database(100).await;

        // Generate random bytes for test data
        let bytes: Vec<u8> = (0..RANDOM_BYTES_SIZE).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);

        // Test fetching existing and none existing header via blockhash and blocknumber from database
        test_get_header(&database, &mongo_fuzzer).await;

        // Test fetching existing and none existing block via blockhash and blocknumber from database
        test_get_blocks(&database, &mongo_fuzzer, &mut unstructured).await;

        // Test fetching existing and none existing transaction counts via blockhash and blocknumber from database
        test_get_transaction_count(&database, &mongo_fuzzer).await;
    }

    async fn test_get_header(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        let header_block_hash = &mongo_fuzzer.headers.first().unwrap().header;

        // Test retrieving header by block hash
        assert_eq!(database.header(header_block_hash.hash.into()).await.unwrap().unwrap(), *header_block_hash);

        // Test retrieving header by block number
        assert_eq!(database.header(header_block_hash.number.into()).await.unwrap().unwrap(), *header_block_hash);

        let mut rng = rand::thread_rng();
        // Test retrieving non-existing header by block hash
        assert_eq!(database.header(rng.gen::<B256>().into()).await.unwrap(), None);

        // Test retrieving non-existing header by block number
        assert_eq!(database.header(rng.gen::<u64>().into()).await.unwrap(), None);
    }

    async fn test_get_blocks(database: &Database, mongo_fuzzer: &MongoFuzzer, u: &mut arbitrary::Unstructured<'_>) {
        let header = &mongo_fuzzer.headers.first().unwrap().header;

        let block_hash = header.hash;

        let block: WithOtherFields<Block<WithOtherFields<Transaction>>> = {
            let transactions: Vec<WithOtherFields<Transaction>> = mongo_fuzzer
                .transactions
                .iter()
                .filter_map(|stored_transaction| {
                    if stored_transaction.block_hash.unwrap() == block_hash {
                        Some(stored_transaction.into())
                    } else {
                        None
                    }
                })
                .collect();

            let block_transactions = BlockTransactions::Full(transactions.clone());

            let signed_transactions =
                transactions.into_iter().map(TransactionSigned::try_from).collect::<Result<Vec<_>, _>>().unwrap();

            let block = reth_primitives::Block {
                body: reth_primitives::BlockBody { transactions: signed_transactions, ..Default::default() },
                header: reth_primitives::Header::try_from(header.clone()).unwrap(),
            };

            let size = block.length();

            WithOtherFields::new(Block {
                header: header.clone(),
                transactions: block_transactions,
                size: Some(U256::from(size)),
                withdrawals: Some(Default::default()),
                ..Default::default()
            })
        };

        // Test retrieving block by block hash
        assert_eq!(database.block(block_hash.into(), true).await.unwrap().unwrap(), block);

        // Test retrieving block by block number
        assert_eq!(database.block(header.number.into(), true).await.unwrap().unwrap(), block);

        let mut rng = rand::thread_rng();

        // Test retrieving non-existing block by block hash
        assert_eq!(database.block(rng.gen::<B256>().into(), false).await.unwrap(), None);

        // Test retrieving non-existing block by block number
        assert_eq!(database.block(rng.gen::<u64>().into(), false).await.unwrap(), None);

        // test withdrawals_root raises an error
        let mut faulty_header = StoredHeader::arbitrary(u).unwrap();
        faulty_header.header.withdrawals_root = Some(rng.gen::<B256>());

        let filter = EthDatabaseFilterBuilder::<filter::Header>::default().with_block_hash(&faulty_header.hash).build();

        database.update_one(faulty_header.clone(), filter, true).await.expect("Failed to update header in database");

        assert!(database.block(faulty_header.hash.into(), true).await.is_err());
    }

    async fn test_get_transaction_count(database: &Database, mongo_fuzzer: &MongoFuzzer) {
        let header_block_hash = &mongo_fuzzer.headers.first().unwrap().header;

        let first_block_hash = header_block_hash.hash;

        let transaction_count: U256 = U256::from(
            mongo_fuzzer
                .transactions
                .iter()
                .filter(|transaction| transaction.tx.block_hash.unwrap() == first_block_hash)
                .count(),
        );

        // Test retrieving header by block hash
        assert_eq!(database.transaction_count(first_block_hash.into()).await.unwrap().unwrap(), transaction_count);

        let mut rng = rand::thread_rng();
        // Test retrieving non-existing transaction count by block hash
        assert_eq!(database.transaction_count(rng.gen::<B256>().into()).await.unwrap(), None);

        // Test retrieving non-existing transaction count by block number
        assert_eq!(database.transaction_count(rng.gen::<u64>().into()).await.unwrap(), None);
    }
}
