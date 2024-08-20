use super::database::types::transaction::StoredPendingTransaction;
use crate::providers::eth_provider::provider::{EthDataProvider, EthProviderResult};
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_rpc_types::{txpool::TxpoolContent, Transaction};
use tracing::Instrument;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait TxPoolProvider {
    /// Returns a vec of pending pool transactions.
    async fn txpool_transactions(&self) -> EthProviderResult<Vec<Transaction>>;

    /// Returns the content of the pending pool.
    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent>;
}

#[async_trait]
impl<SP> TxPoolProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn txpool_transactions(&self) -> EthProviderResult<Vec<Transaction>> {
        let span = tracing::span!(tracing::Level::INFO, "sn::txpool");
        Ok(self.database().get_all_and_map_to::<Transaction, StoredPendingTransaction>().instrument(span).await?)
    }

    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent> {
        Ok(self.txpool_transactions().await?.into_iter().fold(TxpoolContent::default(), |mut content, pending| {
            content.pending.entry(pending.from).or_default().insert(pending.nonce.to_string(), pending);
            content
        }))
    }
}
