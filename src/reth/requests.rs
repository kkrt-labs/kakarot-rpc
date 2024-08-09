use super::KakarotProvider;
use reth_primitives::{BlockHashOrNumber, Requests};
use reth_storage_api::{errors::provider::ProviderResult, RequestsProvider};

impl RequestsProvider for KakarotProvider {
    fn requests_by_block(&self, _id: BlockHashOrNumber, _timestamp: u64) -> ProviderResult<Option<Requests>> {
        Ok(None)
    }
}
