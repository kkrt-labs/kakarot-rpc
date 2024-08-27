use crate::{
    pool::{
        mempool::{KakarotPool, TransactionOrdering},
        validate::KakarotTransactionValidatorBuilder,
    },
    providers::eth_provider::{
        database::{state::EthDatabase, Database},
        error::KakarotError,
        provider::EthDataProvider,
    },
};
use num_traits::ToPrimitive;
use reth_chainspec::ChainSpec;
use reth_transaction_pool::{blobstore::NoopBlobStore, EthPooledTransaction, PoolConfig};
use starknet::{core::types::Felt, providers::Provider};
use std::sync::Arc;

/// Provides a wrapper structure around the Ethereum Provider
/// and the Mempool.
#[derive(Debug, Clone)]
pub struct EthClient<SP: Provider + Send + Sync> {
    eth_provider: EthDataProvider<SP>,
    pool: Arc<KakarotPool<EthDataProvider<SP>>>,
}

impl<SP> EthClient<SP>
where
    SP: Provider + Clone + Sync + Send,
{
    /// Tries to start a [`EthClient`] by fetching the current chain id, initializing a [`EthDataProvider`] and
    /// a `Pool`.
    pub async fn try_new(sn_provider: SP, database: Database) -> eyre::Result<Self> {
        let chain = (sn_provider.chain_id().await.map_err(KakarotError::from)?.to_bigint()
            & Felt::from(u32::MAX).to_bigint())
        .to_u64()
        .unwrap();

        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let mut eth_provider = EthDataProvider::try_new(database, sn_provider).await?;

        let validator =
            KakarotTransactionValidatorBuilder::new(Arc::new(ChainSpec { chain: chain.into(), ..Default::default() }))
                .build::<_, EthPooledTransaction>(EthDatabase::new(eth_provider.clone(), 0.into()));

        let pool = Arc::new(KakarotPool::new(
            validator,
            TransactionOrdering::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        ));

        eth_provider.set_mempool(pool.clone());

        Ok(Self { eth_provider, pool })
    }

    /// Returns a clone of the [`EthDataProvider`]
    pub const fn eth_provider(&self) -> &EthDataProvider<SP> {
        &self.eth_provider
    }

    /// Returns a clone of the `Pool`
    pub fn mempool(&self) -> Arc<KakarotPool<EthDataProvider<SP>>> {
        self.pool.clone()
    }
}
