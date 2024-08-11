use reth_evm::ConfigureEvm;
use reth_rpc::EthApi;
use reth_rpc_eth_api::helpers::FullEthApi;
use reth_rpc_eth_types::{EthStateCache, FeeHistoryCache, FeeHistoryCacheConfig, GasPriceOracle};
use reth_rpc_server_types::constants::{DEFAULT_ETH_PROOF_WINDOW, DEFAULT_PROOF_PERMITS};
use reth_tasks::pool::BlockingTaskPool;

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

#[derive(Debug)]
pub struct KakarotEthApi<Provider, Pool, Network, EvmConfig>(pub EthApi<Provider, Pool, Network, EvmConfig>);

impl<Pool, Network, EvmConfig> KakarotEthApi<KakarotProvider, Pool, Network, EvmConfig>
where
    Pool: Default,
    Network: Default,
    EvmConfig: Default + Clone + ConfigureEvm,
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

impl<Provider, Pool, Network, EvmConfig> Clone for KakarotEthApi<Provider, Pool, Network, EvmConfig> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

#[derive(Debug)]
pub struct KakarotProvider {}

impl Clone for KakarotProvider {
    fn clone(&self) -> Self {
        Self {}
    }
}
