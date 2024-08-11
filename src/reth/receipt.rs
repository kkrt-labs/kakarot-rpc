use super::{KakarotEthApi, KakarotProvider};
use reth_primitives::{BlockHashOrNumber, Receipt, TxHash, TxNumber};
use reth_rpc_eth_api::helpers::LoadReceipt;
use reth_rpc_eth_types::EthStateCache;
use reth_storage_api::{errors::provider::ProviderResult, ReceiptProvider, ReceiptProviderIdExt};

use std::ops::RangeBounds;

impl ReceiptProviderIdExt for KakarotProvider {}

impl ReceiptProvider for KakarotProvider {
    fn receipt(&self, _id: TxNumber) -> ProviderResult<Option<Receipt>> {
        Ok(None)
    }

    fn receipt_by_hash(&self, _hash: TxHash) -> ProviderResult<Option<Receipt>> {
        Ok(None)
    }

    fn receipts_by_block(&self, _block: BlockHashOrNumber) -> ProviderResult<Option<Vec<Receipt>>> {
        Ok(None)
    }

    fn receipts_by_tx_range(&self, _range: impl RangeBounds<TxNumber>) -> ProviderResult<Vec<Receipt>> {
        Ok(vec![])
    }
}

impl<Provider, Pool, Network, EvmConfig> LoadReceipt for KakarotEthApi<Provider, Pool, Network, EvmConfig>
where
    Self: Send + Sync,
{
    #[inline]
    fn cache(&self) -> &EthStateCache {
        self.0.cache()
    }
}
