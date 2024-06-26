use alloy_rlp::Encodable;
use async_trait::async_trait;
use mongodb::bson::doc;
use reth_primitives::constants::EMPTY_ROOT_HASH;
use reth_primitives::{TransactionSigned, B256, U256};
use reth_rpc_types::{Block, BlockHashOrNumber, BlockTransactions, Header, RichBlock, Transaction};

use crate::eth_provider::error::{EthApiError, EthereumDataFormatError};

use super::types::header::StoredHeader;
use super::{filter, FindOpts};
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
    async fn transactions(&self, hash_or_number: BlockHashOrNumber) -> Result<Vec<Transaction>, EthApiError>;
    /// Returns all transactions hashes for the given block hash or number.
    async fn transaction_hashes(&self, hash_or_number: BlockHashOrNumber) -> Result<Vec<B256>, EthApiError>;
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

    async fn transactions(&self, hash_or_number: BlockHashOrNumber) -> Result<Vec<Transaction>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(hash_or_number)
            .build();

        Ok(self.get::<StoredTransaction>(filter, None).await?.into_iter().map(Into::into).collect())
    }

    async fn transaction_hashes(&self, hash_or_number: BlockHashOrNumber) -> Result<Vec<B256>, EthApiError> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(hash_or_number)
            .build();

        Ok(self
            .get::<StoredTransaction>(filter, FindOpts::default().with_projection(doc! {"tx.hash": 1}))
            .await?
            .into_iter()
            .map(|tx| tx.tx.hash)
            .collect())
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
    async fn header(&self, hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError>;
    /// Returns the block for the given hash or number. Returns None if the
    /// block is not found.
    async fn block(&self, hash_or_number: BlockHashOrNumber, full: bool) -> Result<Option<RichBlock>, EthApiError>;
    /// Returns true if the block with the given hash or number exists.
    async fn block_exists(&self, hash_or_number: BlockHashOrNumber) -> Result<bool, EthApiError> {
        self.header(hash_or_number).await.map(|header| header.is_some())
    }
    /// Returns the transaction count for the given block hash or number. Returns None if the
    /// block is not found.
    async fn transaction_count(&self, hash_or_number: BlockHashOrNumber) -> Result<Option<U256>, EthApiError>;
}

#[async_trait]
impl EthereumBlockStore for Database {
    async fn header(&self, hash_or_number: BlockHashOrNumber) -> Result<Option<Header>, EthApiError> {
        let filter =
            EthDatabaseFilterBuilder::<filter::Header>::default().with_block_hash_or_number(hash_or_number).build();
        Ok(self
            .get_one::<StoredHeader>(filter, None)
            .await
            .inspect_err(|err| tracing::error!("internal error: {:?}", err))
            .map_err(|_| EthApiError::UnknownBlock(hash_or_number))?
            .map(|sh| sh.header))
    }

    async fn block(&self, hash_or_number: BlockHashOrNumber, full: bool) -> Result<Option<RichBlock>, EthApiError> {
        let maybe_header = self.header(hash_or_number).await?;
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

        let transactions = self.transactions(hash_or_number).await?;
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

    async fn transaction_count(&self, hash_or_number: BlockHashOrNumber) -> Result<Option<U256>, EthApiError> {
        if !self.block_exists(hash_or_number).await? {
            return Ok(None);
        }

        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash_or_number(hash_or_number)
            .build();
        let count = self.count::<StoredTransaction>(filter).await?;
        Ok(Some(U256::from(count)))
    }
}
