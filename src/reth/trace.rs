use reth_evm::ConfigureEvm;
use reth_rpc_eth_api::helpers::{LoadState, Trace};

use super::KakarotEthApi;

impl<Provider, Pool, Network, EvmConfig> Trace for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: LoadState,
    EvmConfig: ConfigureEvm,
{
    #[inline]
    fn evm_config(&self) -> &impl ConfigureEvm {
        self.0.evm_config()
    }
}
