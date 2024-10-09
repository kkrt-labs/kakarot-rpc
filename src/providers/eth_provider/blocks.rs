use super::{database::ethereum::EthereumBlockStore, error::KakarotError};
use crate::providers::eth_provider::{
    database::ethereum::EthereumTransactionStore,
    provider::{EthApiResult, EthDataProvider},
};
use alloy_primitives::{B256, U256, U64};
use alloy_rpc_types::{Block, Header, Transaction};
use alloy_serde::WithOtherFields;
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{BlockId, BlockNumberOrTag};
use tracing::Instrument;

/// Ethereum block provider trait.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait BlockProvider {
    /// Get header by block id
    async fn header(&self, block_id: &BlockId) -> EthApiResult<Option<Header>>;

    /// Returns the latest block number.
    async fn block_number(&self) -> EthApiResult<U64>;

    /// Returns a block by hash. Block can be full or just the hashes of the transactions.
    async fn block_by_hash(
        &self,
        hash: B256,
        full: bool,
    ) -> EthApiResult<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>>;

    /// Returns a block by number. Block can be full or just the hashes of the transactions.
    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthApiResult<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>>;

    /// Returns the transaction count for a block by hash.
    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthApiResult<Option<U256>>;

    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthApiResult<Option<U256>>;

    /// Returns the transactions for a block.
    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthApiResult<Option<Vec<WithOtherFields<Transaction>>>>;
}

#[async_trait]
impl<SP> BlockProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn header(&self, block_id: &BlockId) -> EthApiResult<Option<Header>> {
        let block_hash_or_number = self.block_id_into_block_number_or_hash(*block_id).await?;
        Ok(self.database().header(block_hash_or_number).await?)
    }

    async fn block_number(&self) -> EthApiResult<U64> {
        let block_number = match self.database().latest_header().await? {
            // In case the database is empty, use the starknet provider
            None => {
                let span = tracing::span!(tracing::Level::INFO, "sn::block_number");
                U64::from(
                    self.starknet_provider_inner().block_number().instrument(span).await.map_err(KakarotError::from)?,
                )
            }
            Some(header) => {
                let is_pending_block = header.hash.is_zero();
                U64::from(if is_pending_block { header.number - 1 } else { header.number })
            }
        };
        Ok(block_number)
    }

    async fn block_by_hash(
        &self,
        hash: B256,
        full: bool,
    ) -> EthApiResult<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>> {
        Ok(self.database().block(hash.into(), full).await?)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthApiResult<Option<WithOtherFields<Block<WithOtherFields<Transaction>>>>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        Ok(self.database().block(block_number.into(), full).await?)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthApiResult<Option<U256>> {
        self.database().transaction_count(hash.into()).await
    }

    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthApiResult<Option<U256>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        self.database().transaction_count(block_number.into()).await
    }

    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthApiResult<Option<Vec<WithOtherFields<Transaction>>>> {
        let block_hash_or_number = self
            .block_id_into_block_number_or_hash(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)))
            .await?;
        if !self.database().block_exists(block_hash_or_number).await? {
            return Ok(None);
        }

        Ok(Some(self.database().transactions(block_hash_or_number).await?))
    }
}
