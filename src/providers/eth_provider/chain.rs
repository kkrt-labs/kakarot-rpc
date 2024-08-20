use crate::providers::eth_provider::{
    error::KakarotError,
    provider::{EthDataProvider, EthProviderResult},
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use reth_primitives::{U256, U64};
use reth_rpc_types::{SyncInfo, SyncStatus};
use starknet::core::types::SyncStatusType;
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait ChainProvider {
    /// Returns the syncing status.
    async fn syncing(&self) -> EthProviderResult<SyncStatus>;

    /// Returns the chain id.
    async fn chain_id(&self) -> EthProviderResult<Option<U64>>;
}

#[async_trait]
impl<SP> ChainProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        let span = tracing::span!(tracing::Level::INFO, "sn::syncing");
        Ok(match self.starknet_provider().syncing().instrument(span).await.map_err(KakarotError::from)? {
            SyncStatusType::NotSyncing => SyncStatus::None,
            SyncStatusType::Syncing(data) => SyncStatus::Info(SyncInfo {
                starting_block: U256::from(data.starting_block_num),
                current_block: U256::from(data.current_block_num),
                highest_block: U256::from(data.highest_block_num),
                ..Default::default()
            }),
        })
    }

    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        Ok(Some(U64::from(self.chain_id)))
    }
}
