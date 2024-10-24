use super::database::{filter::EthDatabaseFilterBuilder, types::receipt::StoredTransactionReceipt};
use crate::providers::eth_provider::{
    database::{
        ethereum::EthereumBlockStore,
        filter::{self},
        types::receipt::ExtendedTxReceipt,
    },
    provider::{EthApiResult, EthDataProvider},
};
use alloy_primitives::B256;
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{BlockId, BlockNumberOrTag};

#[async_trait]
#[auto_impl(Arc, &)]
pub trait ReceiptProvider {
    /// Returns the transaction receipt by hash of the transaction.
    async fn transaction_receipt(&self, hash: B256) -> EthApiResult<Option<ExtendedTxReceipt>>;

    /// Returns the block receipts for a block.
    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthApiResult<Option<Vec<ExtendedTxReceipt>>>;
}

#[async_trait]
impl<SP> ReceiptProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn transaction_receipt(&self, hash: B256) -> EthApiResult<Option<ExtendedTxReceipt>> {
        let filter = EthDatabaseFilterBuilder::<filter::Receipt>::default().with_tx_hash(&hash).build();
        Ok(self.database().get_one::<StoredTransactionReceipt>(filter, None).await?.map(Into::into))
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthApiResult<Option<Vec<ExtendedTxReceipt>>> {
        match block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)) {
            BlockId::Number(number_or_tag) => {
                let block_number = self.tag_into_block_number(number_or_tag).await?;
                if !self.database().block_exists(block_number.into()).await? {
                    return Ok(None);
                }

                let filter =
                    EthDatabaseFilterBuilder::<filter::Receipt>::default().with_block_number(block_number).build();
                let tx: Vec<StoredTransactionReceipt> = self.database().get(filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
            BlockId::Hash(hash) => {
                if !self.database().block_exists(hash.block_hash.into()).await? {
                    return Ok(None);
                }
                let filter =
                    EthDatabaseFilterBuilder::<filter::Receipt>::default().with_block_hash(&hash.block_hash).build();
                Ok(Some(self.database().get_and_map_to::<_, StoredTransactionReceipt>(filter, None).await?))
            }
        }
    }
}
