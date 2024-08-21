use super::{
    constant::MAX_LOGS,
    database::{filter::EthDatabaseFilterBuilder, types::log::StoredLog},
    error::EthApiError,
};
use crate::providers::eth_provider::{
    database::{
        filter::{self},
        FindOpts,
    },
    provider::{EthDataProvider, EthProviderResult},
    BlockProvider,
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use reth_rpc_types::{Filter, FilterChanges};

#[async_trait]
#[auto_impl(Arc, &)]
pub trait LogProvider: BlockProvider {
    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges>;
}

#[async_trait]
impl<SP> LogProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        let block_hash = filter.get_block_hash();

        // Create the database filter.
        let mut builder = EthDatabaseFilterBuilder::<filter::Log>::default();
        builder = if block_hash.is_some() {
            // We filter by block hash on matching the exact block hash.
            builder.with_block_hash(&block_hash.unwrap())
        } else {
            let current_block = self.block_number().await?;
            let current_block =
                current_block.try_into().map_err(|_| EthApiError::UnknownBlockNumber(Some(current_block.to())))?;

            let from = filter.get_from_block().unwrap_or_default();
            let to = filter.get_to_block().unwrap_or(current_block);

            let (from, to) = match (from, to) {
                (from, to) if from > current_block || to < from => return Ok(FilterChanges::Empty),
                (from, to) if to > current_block => (from, current_block),
                other => other,
            };
            // We filter by block number using $gte and $lte.
            builder.with_block_number_range(from, to)
        };

        // TODO: this will work for now but isn't very efficient. Would need to:
        // 1. Create the bloom filter from the topics
        // 2. Query the database for logs within block range with the bloom filter
        // 3. Filter this reduced set of logs by the topics
        // 4. Limit the number of logs returned

        // Convert the topics to a MongoDB filter and add it to the database filter
        builder = builder.with_topics(&filter.topics);

        // Add the addresses
        builder = builder.with_addresses(&filter.address.into_iter().collect::<Vec<_>>());

        Ok(FilterChanges::Logs(
            self.database()
                .get_and_map_to::<_, StoredLog>(
                    builder.build(),
                    (*MAX_LOGS).map(|limit| FindOpts::default().with_limit(limit)),
                )
                .await?,
        ))
    }
}
