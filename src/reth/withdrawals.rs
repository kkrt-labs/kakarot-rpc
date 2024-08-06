use super::KakarotProvider;
use reth_primitives::BlockHashOrNumber;
use reth_storage_api::{errors::provider::ProviderResult, WithdrawalsProvider};

impl WithdrawalsProvider for KakarotProvider {
    fn withdrawals_by_block(
        &self,
        _id: BlockHashOrNumber,
        _timestamp: u64,
    ) -> ProviderResult<Option<reth_primitives::Withdrawals>> {
        Ok(None)
    }

    fn latest_withdrawal(&self) -> ProviderResult<Option<reth_primitives::Withdrawal>> {
        Ok(None)
    }
}
