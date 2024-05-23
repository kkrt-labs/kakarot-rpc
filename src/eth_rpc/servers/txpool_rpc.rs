use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::txpool_api::TxPoolApiServer;
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::Address;
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus};
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
    /// Returns a summary of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_inspect) for more details
    ///
    /// Handler for `txpool_inspect`
    async fn txpool_inspect(&self) -> Result<TxpoolInspect> {
        trace!(target: "rpc::eth", "Serving txpool_inspect");

        let mut inspect = TxpoolInspect::default();

        let transactions = self.eth_provider.txpool_transactions().await?;

        for transaction in transactions {
            inspect.pending.entry(transaction.from).or_default().insert(
                transaction.nonce.to_string(),
                TxpoolInspectSummary {
                    to: transaction.to,
                    value: transaction.value,
                    gas: transaction.gas,
                    gas_price: transaction.gas_price.unwrap_or_default(),
                },
            );
        }

        Ok(inspect)
    }

    /// Returns the number of transactions currently pending for inclusion in the next block(s), as
    /// well as the ones that are being scheduled for future execution only.
    /// Ref: [Here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_status)
    ///
    /// Handler for `txpool_status`
    async fn txpool_status(&self) -> Result<TxpoolStatus> {
        trace!(target: "rpc::eth", "Serving txpool_status");
        let all = self.eth_provider.txpool_content().await?;
        Ok(TxpoolStatus { pending: all.pending.len() as u64, queued: all.queued.len() as u64 })
    }

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
