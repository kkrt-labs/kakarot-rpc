use reth_provider::{BlockIdReader, BlockReaderIdExt, ChainSpecProvider, HeaderProvider};
use reth_rpc_eth_api::helpers::{EthFees, LoadBlock, LoadFee};
use reth_rpc_eth_types::{EthStateCache, FeeHistoryCache, GasPriceOracle};

use super::KakarotEthApi;

impl<Provider, Pool, Network, EvmConfig> EthFees for KakarotEthApi<Provider, Pool, Network, EvmConfig> where
    Self: LoadFee
{
}

impl<Provider, Pool, Network, EvmConfig> LoadFee for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: LoadBlock,
    Provider: BlockReaderIdExt + HeaderProvider + ChainSpecProvider,
{
    #[inline]
    fn provider(&self) -> impl BlockIdReader + HeaderProvider + ChainSpecProvider {
        self.0.provider()
    }

    #[inline]
    fn cache(&self) -> &EthStateCache {
        self.0.cache()
    }

    #[inline]
    fn gas_oracle(&self) -> &GasPriceOracle<impl BlockReaderIdExt> {
        self.0.gas_oracle()
    }

    #[inline]
    fn fee_history_cache(&self) -> &FeeHistoryCache {
        self.0.fee_history_cache()
    }
}
