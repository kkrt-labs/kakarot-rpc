use crate::{eth_provider::provider::EthereumProvider, eth_rpc::api::txpool_api::TxPoolApiServer};
use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::Address;
use reth_rpc_types::txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus};
use tracing::instrument;

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
    /// Returns the number of transactions currently pending for inclusion in the next block(s), as
    /// well as the ones that are being scheduled for future execution only.
    /// Ref: [Here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_status)
    ///
    /// Handler for `txpool_status`
    #[instrument(skip(self))]
    async fn txpool_status(&self) -> Result<TxpoolStatus> {
        let all = self.eth_provider.txpool_content().await?;
        Ok(TxpoolStatus { pending: all.pending.len() as u64, queued: all.queued.len() as u64 })
    }

    /// Returns a summary of all the transactions currently pending for inclusion in the next
    /// block(s), as well as the ones that are being scheduled for future execution only.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_inspect) for more details
    ///
    /// Handler for `txpool_inspect`
    #[instrument(skip(self))]
    async fn txpool_inspect(&self) -> Result<TxpoolInspect> {
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

    /// Retrieves the transactions contained within the txpool, returning pending
    /// transactions of this address, grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_contentFrom) for more details
    /// Handler for `txpool_contentFrom`
    #[instrument(skip(self))]
    async fn txpool_content_from(&self, from: Address) -> Result<TxpoolContentFrom> {
        Ok(self.eth_provider.txpool_content().await?.remove_from(&from))
    }

    /// Returns the details of all transactions currently pending for inclusion in the next
    /// block(s), grouped by nonce.
    ///
    /// See [here](https://geth.ethereum.org/docs/rpc/ns-txpool#txpool_content) for more details
    /// Handler for `txpool_content`
    #[instrument(skip(self))]
    async fn txpool_content(&self) -> Result<TxpoolContent> {
        Ok(self.eth_provider.txpool_content().await?)
    }
}
