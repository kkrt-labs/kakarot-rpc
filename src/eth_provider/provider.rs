use async_trait::async_trait;
use auto_impl::auto_impl;
use eyre::Result;
use mockall::automock;
use mongodb::bson::doc;
use mongodb::bson::Document;
use reth_primitives::{BlockNumberOrTag, H256, U256, U64};
use reth_rpc_types::Index;
use reth_rpc_types::Transaction;
use reth_rpc_types::{Block, BlockTransactions, RichBlock};
use reth_rpc_types::{SyncInfo, SyncStatus};
use starknet::core::types::SyncStatusType;
use starknet::providers::Provider as StarknetProvider;

use super::database::Database;
use super::types::header::StoredHeader;
use super::types::transaction::StoredTransactionHash;
use super::utils::iter_into;
use super::{error::EthProviderError, types::transaction::StoredTransactionFull, utils::into_filter};

pub type EthProviderResult<T> = Result<T, EthProviderError>;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc)]
#[automock]
pub trait EthereumProvider {
    /// Returns the latest block number.
    async fn block_number(&self) -> EthProviderResult<U64>;
    /// Returns the syncing status.
    async fn syncing(&self) -> EthProviderResult<SyncStatus>;
    /// Returns the chain id.
    async fn chain_id(&self) -> EthProviderResult<Option<U64>>;
    /// Returns a block by hash. Block can be full or just the hashes of the transactions.
    async fn block_by_hash(&self, hash: H256, full: bool) -> EthProviderResult<Option<RichBlock>>;
    /// Returns a block by number. Block can be full or just the hashes of the transactions.
    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>>;
    /// Returns the transaction count for a block by hash.
    async fn block_transaction_count_by_hash(&self, hash: H256) -> EthProviderResult<U64>;
    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64>;
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: H256) -> EthProviderResult<Option<Transaction>>;
    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> EthProviderResult<Option<Transaction>>;
}

/// Structure that implements the EthereumProvider trait.
/// Uses an access to a database to certain data, while
/// the rest is fetched from the Starknet Provider.
pub struct EthereumAccessLayer<SP>
where
    SP: StarknetProvider + Send + Sync,
{
    database: Database,
    starknet_provider: SP,
}

#[async_trait]
impl<SP> EthereumProvider for EthereumAccessLayer<SP>
where
    SP: StarknetProvider + Send + Sync,
{
    async fn block_number(&self) -> EthProviderResult<U64> {
        let filter = doc! {};
        let sort = doc! { "header.number": -1 };
        let header: Option<StoredHeader> = self.database.get_one("headers", filter, sort).await?;
        let block_number = match header {
            Some(header) => header.header.number.ok_or(EthProviderError::ValueNotFound)?.as_limbs()[0],
            None => self.starknet_provider.block_number().await?, // in case the database is empty, use the starknet provider
        };
        Ok(block_number.into())
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        let syncing_status = self.starknet_provider.syncing().await?;

        match syncing_status {
            SyncStatusType::NotSyncing => Ok(SyncStatus::None),

            SyncStatusType::Syncing(data) => {
                let starting_block: U256 = U256::from(data.starting_block_num);
                let current_block: U256 = U256::from(data.current_block_num);
                let highest_block: U256 = U256::from(data.highest_block_num);

                let status_info = SyncInfo {
                    starting_block,
                    current_block,
                    highest_block,
                    warp_chunks_amount: None,
                    warp_chunks_processed: None,
                };

                Ok(SyncStatus::Info(status_info))
            }
        }
    }

    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        let chain_id = self.starknet_provider.chain_id().await?;
        let chain_id: Option<u64> = chain_id.try_into().ok();
        Ok(chain_id.map(Into::into))
    }

    async fn block_by_hash(&self, hash: H256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        let header_filter = into_filter("header.hash", hash, 64);
        let tx_filter = into_filter("tx.blockHash", hash, 64);
        let block = self.block(header_filter, tx_filter, full).await?;

        Ok(block)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;

        let header_filter = into_filter("header.number", block_number, 64);
        let tx_filter = into_filter("tx.blockNumber", block_number, 64);
        let block = self.block(header_filter, tx_filter, full).await?;

        Ok(block)
    }

    async fn block_transaction_count_by_hash(&self, hash: H256) -> EthProviderResult<U64> {
        let filter = into_filter("tx.blockHash", hash, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(count.into())
    }

    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;

        let filter = into_filter("tx.blockNumber", block_number, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(count.into())
    }

    async fn transaction_by_hash(&self, hash: H256) -> EthProviderResult<Option<Transaction>> {
        let filter = into_filter("tx.hash", hash, 64);
        let tx: Option<StoredTransactionFull> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(|tx| tx.tx))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> EthProviderResult<Option<Transaction>> {
        let index: usize = index.into();
        let mut filter = into_filter("tx.blockHash", hash, 64);
        filter.insert("tx.transactionIndex", index as i32);
        let tx: Option<StoredTransactionFull> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(|tx| tx.tx))
    }
}

impl<SP> EthereumAccessLayer<SP>
where
    SP: StarknetProvider + Send + Sync,
{
    pub fn new(database: Database, starknet_provider: SP) -> Self {
        Self { database, starknet_provider }
    }

    /// Get a block from the database based on the header and transaction filters
    /// If full is true, the block will contain the full transactions, otherwise just the hashes
    async fn block(
        &self,
        header_filter: impl Into<Option<Document>>,
        transactions_filter: impl Into<Option<Document>>,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let header = self.database.get_one::<StoredHeader>("headers", header_filter, None).await?;
        let header = match header {
            Some(header) => header,
            None => return Ok(None),
        };
        let total_difficulty = Some(header.header.difficulty);

        let transactions = if full {
            BlockTransactions::Full(iter_into(
                self.database.get::<StoredTransactionFull>("transactions", transactions_filter, None).await?,
            ))
        } else {
            BlockTransactions::Hashes(iter_into(
                self.database
                    .get::<StoredTransactionHash>("transactions", transactions_filter, doc! {"tx.blockHash": 1})
                    .await?,
            ))
        };

        let block = Block {
            header: header.header,
            transactions,
            total_difficulty,
            uncles: Vec::new(),
            size: None,
            withdrawals: None,
        };

        Ok(Some(block.into()))
    }

    /// Convert the given BlockNumberOrTag into a block number
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        match tag {
            BlockNumberOrTag::Earliest => Ok(U64::zero()),
            BlockNumberOrTag::Number(number) => Ok(number.into()),
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await
            }
            BlockNumberOrTag::Pending => todo!("pending block number not implemented"),
        }
    }
}
