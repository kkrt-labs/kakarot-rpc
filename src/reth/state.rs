use super::KakarotProvider;
use reth_primitives::{Account, Address, BlockHash, BlockNumber, Bytecode, StorageKey, StorageValue, B256};
use reth_revm::db::BundleState;
use reth_storage_api::{
    errors::provider::ProviderResult, AccountReader, StateProofProvider, StateProvider, StateProviderBox,
    StateProviderFactory, StateRootProvider,
};
use reth_trie::updates::TrieUpdates;
use reth_trie_common::AccountProof;

impl StateProofProvider for KakarotProvider {
    fn proof(&self, _state: &BundleState, _address: Address, _slots: &[B256]) -> ProviderResult<AccountProof> {
        Ok(AccountProof::new(Address::default()))
    }
}

impl StateRootProvider for KakarotProvider {
    fn state_root(&self, _bundle_state: &BundleState) -> ProviderResult<B256> {
        Ok(B256::default())
    }

    fn state_root_with_updates(&self, _bundle_state: &BundleState) -> ProviderResult<(B256, TrieUpdates)> {
        Ok((B256::default(), TrieUpdates::default()))
    }
}

impl AccountReader for KakarotProvider {
    fn basic_account(&self, _address: Address) -> ProviderResult<Option<Account>> {
        Ok(None)
    }
}

impl StateProvider for KakarotProvider {
    fn storage(&self, _account: Address, _storage_key: StorageKey) -> ProviderResult<Option<StorageValue>> {
        Ok(None)
    }

    fn bytecode_by_hash(&self, _code_hash: B256) -> ProviderResult<Option<Bytecode>> {
        Ok(None)
    }
}

impl StateProviderFactory for KakarotProvider {
    fn latest(&self) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }

    fn history_by_block_number(&self, _block: BlockNumber) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }

    fn history_by_block_hash(&self, _block: BlockHash) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }

    fn state_by_block_hash(&self, _block: BlockHash) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }

    fn pending(&self) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }

    fn pending_state_by_hash(&self, _block_hash: B256) -> ProviderResult<Option<StateProviderBox>> {
        Ok(Some(Box::new(self.clone())))
    }

    fn pending_with_provider(
        &self,
        _bundle_state_data: Box<dyn reth_storage_api::FullExecutionDataProvider>,
    ) -> ProviderResult<StateProviderBox> {
        Ok(Box::new(self.clone()))
    }
}
