use super::eth_provider::TxPoolProvider;
use crate::providers::eth_provider::provider::EthApiResult;
use alloy_primitives::Address;
use alloy_rpc_types::Transaction;
use alloy_rpc_types_txpool::{TxpoolContent, TxpoolContentFrom, TxpoolInspect, TxpoolInspectSummary, TxpoolStatus};
use alloy_serde::WithOtherFields;
use async_trait::async_trait;
use auto_impl::auto_impl;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait PoolProvider {
    async fn txpool_status(&self) -> EthApiResult<TxpoolStatus>;
    async fn txpool_inspect(&self) -> EthApiResult<TxpoolInspect>;
    async fn txpool_content_from(&self, from: Address)
        -> EthApiResult<TxpoolContentFrom<WithOtherFields<Transaction>>>;
    async fn txpool_content(&self) -> EthApiResult<TxpoolContent<WithOtherFields<Transaction>>>;
}

#[derive(Debug, Clone)]
pub struct PoolDataProvider<P: TxPoolProvider> {
    eth_provider: P,
}

impl<P: TxPoolProvider> PoolDataProvider<P> {
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P: TxPoolProvider + Send + Sync + 'static> PoolProvider for PoolDataProvider<P> {
    async fn txpool_status(&self) -> EthApiResult<TxpoolStatus> {
        let all = self.eth_provider.txpool_content().await?;
        Ok(TxpoolStatus { pending: all.pending.len() as u64, queued: all.queued.len() as u64 })
    }

    async fn txpool_inspect(&self) -> EthApiResult<TxpoolInspect> {
        let mut inspect = TxpoolInspect::default();

        let transactions = self.eth_provider.content();

        // Organize the pending transactions in the inspect summary struct.
        for (sender, nonce_transaction) in transactions.pending {
            for (nonce, transaction) in nonce_transaction {
                inspect.pending.entry((*sender).into()).or_default().insert(
                    nonce.clone(),
                    TxpoolInspectSummary {
                        to: transaction.to,
                        value: transaction.value,
                        gas: transaction.gas.into(),
                        gas_price: transaction.gas_price.unwrap_or_default(),
                    },
                );
            }
        }

        // Organize the queued transactions in the inspect summary struct.
        for (sender, nonce_transaction) in transactions.queued {
            for (nonce, transaction) in nonce_transaction {
                inspect.queued.entry((*sender).into()).or_default().insert(
                    nonce.clone(),
                    TxpoolInspectSummary {
                        to: transaction.to,
                        value: transaction.value,
                        gas: transaction.gas.into(),
                        gas_price: transaction.gas_price.unwrap_or_default(),
                    },
                );
            }
        }

        Ok(inspect)
    }

    async fn txpool_content_from(
        &self,
        from: Address,
    ) -> EthApiResult<TxpoolContentFrom<WithOtherFields<Transaction>>> {
        Ok(self.eth_provider.txpool_content().await?.remove_from(&from))
    }

    async fn txpool_content(&self) -> EthApiResult<TxpoolContent<WithOtherFields<Transaction>>> {
        Ok(self.eth_provider.txpool_content().await?)
    }
}
