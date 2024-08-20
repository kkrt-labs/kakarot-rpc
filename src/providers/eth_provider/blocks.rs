use super::{
    database::ethereum::EthereumBlockStore,
    error::{EthApiError, KakarotError},
};
use crate::providers::eth_provider::{
    database::ethereum::EthereumTransactionStore,
    provider::{EthDataProvider, EthProviderResult},
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{BlockId, BlockNumberOrTag, B256, U256, U64};
use reth_rpc_types::{Header, RichBlock};
use tracing::Instrument;

/// Ethereum block provider trait.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait BlockProvider {
    /// Get header by block id
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>>;

    /// Returns the latest block number.
    async fn block_number(&self) -> EthProviderResult<U64>;

    /// Returns a block by hash. Block can be full or just the hashes of the transactions.
    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>>;

    /// Returns a block by number. Block can be full or just the hashes of the transactions.
    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>>;

    /// Returns the transaction count for a block by hash.
    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>>;

    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>>;

    /// Returns the transactions for a block.
    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>>;
}

#[async_trait]
impl<SP> BlockProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>> {
        let block_hash_or_number = self.block_id_into_block_number_or_hash(*block_id).await?;
        Ok(self.database().header(block_hash_or_number).await?)
    }

    async fn block_number(&self) -> EthProviderResult<U64> {
        let block_number = match self.database().latest_header().await? {
            // In case the database is empty, use the starknet provider
            None => {
                let span = tracing::span!(tracing::Level::INFO, "sn::block_number");
                U64::from(self.starknet_provider().block_number().instrument(span).await.map_err(KakarotError::from)?)
            }
            Some(header) => {
                let number = header.number.ok_or(EthApiError::UnknownBlockNumber(None))?;
                let is_pending_block = header.hash.unwrap_or_default().is_zero();
                U64::from(if is_pending_block { number - 1 } else { number })
            }
        };
        Ok(block_number)
    }

    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        Ok(self.database().block(hash.into(), full).await?)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        Ok(self.database().block(block_number.into(), full).await?)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        self.database().transaction_count(hash.into()).await
    }

    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        self.database().transaction_count(block_number.into()).await
    }

    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>> {
        let block_hash_or_number = self
            .block_id_into_block_number_or_hash(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)))
            .await?;
        if !self.database().block_exists(block_hash_or_number).await? {
            return Ok(None);
        }

        Ok(Some(self.database().transactions(block_hash_or_number).await?))
    }
}
