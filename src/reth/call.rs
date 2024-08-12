use reth_evm::ConfigureEvm;
use reth_rpc_eth_api::helpers::{Call, EthCall, LoadPendingBlock, LoadState, SpawnBlocking};

use super::KakarotEthApi;

impl<Provider, Pool, Network, EvmConfig> EthCall for KakarotEthApi<Provider, Pool, Network, EvmConfig> where
    Self: Call + LoadPendingBlock
{
}

impl<Provider, Pool, Network, EvmConfig> Call for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: LoadState + SpawnBlocking,
    EvmConfig: ConfigureEvm,
{
    #[inline]
    fn call_gas_limit(&self) -> u64 {
        self.0.gas_cap()
    }

    #[inline]
    fn evm_config(&self) -> &impl ConfigureEvm {
        self.0.evm_config()
    }
}
