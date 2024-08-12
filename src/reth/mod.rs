use reth_chainspec::ChainSpec;
use reth_evm::ConfigureEvm;
use reth_provider::ChainSpecProvider;
use reth_rpc::EthApi;
use reth_rpc_eth_types::{EthStateCache, FeeHistoryCache, FeeHistoryCacheConfig, GasPriceOracle};
use reth_rpc_server_types::constants::{DEFAULT_ETH_PROOF_WINDOW, DEFAULT_PROOF_PERMITS};
use reth_tasks::pool::BlockingTaskPool;
use std::sync::Arc;

pub mod block;
pub mod call;
pub mod evm;
pub mod fees;
pub mod receipt;
pub mod requests;
pub mod spawn;
pub mod spec;
pub mod state;
pub mod trace;
pub mod transaction;
pub mod withdrawals;

#[derive(Debug, Clone)]
pub struct KakarotEthApi<Provider, Pool, Network, EvmConfig>(pub EthApi<Provider, Pool, Network, EvmConfig>);

impl<Pool, Network, EvmConfig> KakarotEthApi<KakarotProvider, Pool, Network, EvmConfig>
where
    EvmConfig: Clone + ConfigureEvm,
{
    pub fn new(pool: Pool, network: Network, evm_config: EvmConfig) -> Self {
        let provider = KakarotProvider {};
        let cache = EthStateCache::spawn(provider.clone(), Default::default(), evm_config.clone());
        let fee_history_cache = FeeHistoryCache::new(cache.clone(), FeeHistoryCacheConfig::default());

        let gas_cap = u64::MAX;
        Self(EthApi::new(
            provider.clone(),
            pool,
            network,
            cache.clone(),
            GasPriceOracle::new(provider, Default::default(), cache),
            gas_cap,
            DEFAULT_ETH_PROOF_WINDOW,
            BlockingTaskPool::build().expect("failed to build tracing pool"),
            fee_history_cache,
            evm_config,
            None,
            DEFAULT_PROOF_PERMITS,
        ))
    }
}

#[derive(Debug, Clone)]
pub struct KakarotProvider {}

impl ChainSpecProvider for KakarotProvider {
    fn chain_spec(&self) -> Arc<ChainSpec> {
        Arc::new(ChainSpec::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth_evm_ethereum::EthEvmConfig;
    use reth_rpc_eth_api::helpers::FullEthApi;
    use reth_transaction_pool::noop::NoopTransactionPool;

    fn is_full_eth_api<F: FullEthApi>(_: F) {}

    #[test]
    fn test_is_full_eth_api() {
        let pool = NoopTransactionPool::default();
        let network = ();
        let config = EthEvmConfig::default();

        let eth_api = KakarotEthApi::new(pool, network, config);
        is_full_eth_api(eth_api);
    }
}
