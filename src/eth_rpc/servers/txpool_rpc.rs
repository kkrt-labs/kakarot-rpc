use crate::{eth_rpc::api::txpool_api::TxPoolApiServer, providers::pool_provider::provider::PoolProvider};
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::Address;
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolStatus};
use tracing::instrument;

/// The RPC module for implementing the Txpool api
#[derive(Debug)]
pub struct TxpoolRpc<PP: PoolProvider> {
    pool_provider: PP,
}

impl<PP> TxpoolRpc<PP>
where
    PP: PoolProvider,
{
    pub const fn new(pool_provider: PP) -> Self {
        Self { pool_provider }
    }
}

#[async_trait]
impl<PP> TxPoolApiServer for TxpoolRpc<PP>
where
    PP: PoolProvider + Send + Sync + 'static,
{
    /// Returns the number of transactions currently pending for inclusion in the next block(s), as
    /// well as the ones that are being scheduled for future execution only.
    /// Ref: [Here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_status)
    ///
    /// Handler for `txpool_status`
    #[instrument(skip(self))]
    async fn txpool_status(&self) -> Result<TxpoolStatus> {
        self.pool_provider.txpool_status().await.map_err(Into::into)
    }

    /// Returns a summary of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_inspect) for more details
    ///
    /// Handler for `txpool_inspect`
    #[instrument(skip(self))]
    async fn txpool_inspect(&self) -> Result<TxpoolInspect> {
        self.pool_provider.txpool_inspect().await.map_err(Into::into)
    }

    /// Retrieves the transactions contained within the txpool, returning pending
    /// transactions of this address, grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    /// Handler for `txpool_contentFrom`
    #[instrument(skip(self))]
    async fn txpool_content_from(&self, from: Address) -> Result<TxpoolContentFrom> {
        self.pool_provider.txpool_content_from(from).await.map_err(Into::into)
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s), grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    /// Handler for `txpool_content`
    #[instrument(skip(self))]
    async fn txpool_content(&self) -> Result<TxpoolContent> {
        self.pool_provider.txpool_content().await.map_err(Into::into)
    }
}
