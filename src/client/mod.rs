use crate::{
    pool::{
        mempool::{KakarotPool, TransactionOrdering},
        validate::KakarotTransactionValidatorBuilder,
    },
    providers::eth_provider::{
        database::state::EthDatabase,
        provider::{EthApiResult, EthereumProvider},
    },
};
use reth_chainspec::ChainSpec;
use reth_transaction_pool::{blobstore::NoopBlobStore, EthPooledTransaction, PoolConfig};
use std::sync::Arc;

#[derive(Debug)]
pub struct EthClient<EP: EthereumProvider + Send + Sync> {
    provider: EP,
    pool: KakarotPool<EP>,
}

impl<EP> EthClient<EP>
where
    EP: EthereumProvider + Send + Sync + Clone,
{
    pub async fn try_new(eth_provider: EP) -> EthApiResult<Self> {
        let chain: u64 = eth_provider.chain_id().await?.unwrap_or_default().to();
        let validator =
            KakarotTransactionValidatorBuilder::new(Arc::new(ChainSpec { chain: chain.into(), ..Default::default() }))
                .build::<_, EthPooledTransaction>(EthDatabase::new(eth_provider.clone(), 0.into()));

        let mempool = KakarotPool::new(
            validator,
            TransactionOrdering::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        );
        Ok(Self { provider: eth_provider, pool: mempool })
    }
}
