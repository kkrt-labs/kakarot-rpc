use super::database::types::transaction::ExtendedTransaction;
use crate::providers::eth_provider::provider::EthApiResult;
use alloy_rpc_types_txpool::TxpoolContent;
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait TxPoolProvider {
    /// Returns a vec of pending pool transactions.
    fn content(&self) -> TxpoolContent<ExtendedTransaction>;

    /// Returns the content of the pending pool.
    async fn txpool_content(&self) -> EthApiResult<TxpoolContent<ExtendedTransaction>>;
}
