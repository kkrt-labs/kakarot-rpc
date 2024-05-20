use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::txpool_api::TxPoolApiServer;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::Address;
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom};
use tracing::trace;

/// The RPC module for implementing the Txpool api
#[derive(Debug)]
pub struct TxpoolRpc<P: EthereumProvider> {
    eth_provider: P,
}

impl<P: EthereumProvider> TxpoolRpc<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: EthereumProvider + Send + Sync + 'static> TxPoolApiServer for TxpoolRpc<P> {
    /// Retrieves the transactions contained within the txpool, returning pending
    /// transactions of this address, grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    /// Handler for `txpool_contentFrom`
    async fn txpool_content_from(&self, from: Address) -> Result<TxpoolContentFrom> {
        trace!(target: "rpc::eth", ?from, "Serving txpool_contentFrom");
        Ok(self.eth_provider.txpool_content().await?.remove_from(&from))
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s), grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    /// Handler for `txpool_content`
    async fn txpool_content(&self) -> Result<TxpoolContent> {
        trace!(target: "rpc::eth", "Serving txpool_content");
        Ok(self.eth_provider.txpool_content().await?)
    }
}
