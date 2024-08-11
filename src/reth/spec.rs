use super::KakarotEthApi;
use reth_chainspec::ChainInfo;
use reth_errors::RethResult;
use reth_evm::ConfigureEvm;
use reth_primitives::{Address, U64};
use reth_rpc_eth_api::helpers::EthApiSpec;
use reth_rpc_types::SyncStatus;
use reth_storage_api::BlockReaderIdExt;

impl<Provider, Pool, Network, EvmConfig> EthApiSpec for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Pool: Send + Sync + 'static,
    Provider: BlockReaderIdExt + 'static,
    Network: Send + Sync + 'static,
    EvmConfig: ConfigureEvm,
{
    /// Returns the current ethereum protocol version.
    ///
    /// Note: This returns an [`U64`], since this should return as hex string.
    async fn protocol_version(&self) -> RethResult<U64> {
        Ok(Default::default())
    }

    /// Returns the chain id
    fn chain_id(&self) -> U64 {
        Default::default()
    }

    /// Returns the current info for the chain
    fn chain_info(&self) -> RethResult<ChainInfo> {
        Ok(Default::default())
    }

    fn accounts(&self) -> Vec<Address> {
        vec![]
    }

    fn is_syncing(&self) -> bool {
        Default::default()
    }

    /// Returns the [`SyncStatus`] of the network
    fn sync_status(&self) -> RethResult<SyncStatus> {
        Ok(SyncStatus::None)
    }
}
