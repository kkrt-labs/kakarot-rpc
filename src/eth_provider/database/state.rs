use crate::eth_provider::{error::EthApiError, provider::EthereumProvider};
use reth_primitives::{Address, B256, U256};
use reth_revm::{
    db::{AccountState, CacheDB, DbAccount},
    primitives::{Account, AccountInfo, Bytecode},
    Database, DatabaseCommit, DatabaseRef,
};
use reth_rpc_types::{serde_helpers::JsonStorageKey, BlockHashOrNumber, BlockId, BlockNumberOrTag};
use std::collections::HashMap;
use tokio::runtime::Handle;

#[derive(Debug)]
pub struct EthCacheDatabase<P: EthereumProvider + Send + Sync>(pub CacheDB<EthDatabase<P>>);

/// Ethereum database type.
#[derive(Debug)]
#[allow(clippy::redundant_pub_crate)]
pub struct EthDatabase<P: EthereumProvider + Send + Sync> {
    /// The Ethereum provider.
    provider: P,
    /// The block ID.
    block_id: BlockId,
}

impl<P: EthereumProvider + Send + Sync> EthDatabase<P> {
    pub(crate) const fn new(provider: P, block_id: BlockId) -> Self {
        Self { provider, block_id }
    }
}

impl<P: EthereumProvider + Send + Sync> DatabaseRef for EthDatabase<P> {
    type Error = EthApiError;

    /// Returns the account information for the given address without caching.
    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        tokio::task::block_in_place(|| {
            let account_info = Handle::current().block_on(async {
                let bytecode = self.provider.get_code(address, Some(self.block_id)).await?;
                let bytecode = Bytecode::new_raw(bytecode);
                let code_hash = bytecode.hash_slow();

                let nonce = self.provider.transaction_count(address, Some(self.block_id)).await?.to();
                let balance = self.provider.balance(address, Some(self.block_id)).await?;

                Result::<_, EthApiError>::Ok(AccountInfo { nonce, balance, code: Some(bytecode), code_hash })
            })?;

            Ok(Some(account_info))
        })
    }

    /// Returns the code for the given code hash.
    /// TODO: Implement this method in the provider
    fn code_by_hash_ref(&self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(Default::default())
    }

    /// Returns the storage value for the given address and index without caching.
    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Self::Error> {
        tokio::task::block_in_place(|| {
            let storage = Handle::current().block_on(async {
                let value = self
                    .provider
                    .storage_at(
                        address,
                        JsonStorageKey(B256::from_slice(&index.to_be_bytes::<32>())),
                        Some(self.block_id),
                    )
                    .await?;
                Result::<_, EthApiError>::Ok(value)
            })?;
            let storage = U256::from_be_bytes(storage.0);

            Ok(storage)
        })
    }

    /// Returns the block hash for the given block number without caching.
    fn block_hash_ref(&self, block_number: u64) -> Result<B256, Self::Error> {
        tokio::task::block_in_place(|| {
            let hash = Handle::current().block_on(async {
                let hash = self
                    .provider
                    .block_by_number(BlockNumberOrTag::Number(block_number), false)
                    .await?
                    .ok_or(EthApiError::UnknownBlock(BlockHashOrNumber::Number(block_number)))?
                    .header
                    .hash
                    .unwrap_or_default();
                Result::<_, EthApiError>::Ok(hash)
            })?;

            Ok(hash)
        })
    }
}

impl<P: EthereumProvider + Send + Sync> Database for EthCacheDatabase<P> {
    type Error = EthApiError;

    /// Returns the account information for the given address.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-async runtime.
    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        if let Some(account) = self.0.accounts.get(&address) {
            return Ok(Some(account.info.clone()));
        }

        let account_info = DatabaseRef::basic_ref(&self.0, address)?;
        self.0.insert_account_info(address, account_info.clone().unwrap_or_default());

        Ok(account_info)
    }

    /// Returns the code for the given code hash.
    /// TODO: Implement this method in the provider
    fn code_by_hash(&mut self, _code_hash: B256) -> Result<Bytecode, Self::Error> {
        Ok(Bytecode::default())
    }

    /// Returns the storage value for the given address and index.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-async runtime.
    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        if let Some(account) = self.0.accounts.get(&address) {
            return Ok(account.storage.get(&index).copied().unwrap_or_default());
        }

        let storage = DatabaseRef::storage_ref(&self.0, address, index)?;

        self.0.accounts.entry(address).or_default().storage.insert(index, storage);
        Ok(storage)
    }

    /// Returns the block hash for the given block number.
    ///
    /// # Panics
    ///
    /// Panics if called from a non-async runtime.
    fn block_hash(&mut self, block_number: u64) -> Result<B256, Self::Error> {
        let number = U256::from(block_number);
        if let Some(hash) = self.0.block_hashes.get(&number) {
            return Ok(*hash);
        }

        let hash = DatabaseRef::block_hash_ref(&self.0, block_number)?;
        self.0.block_hashes.insert(number, hash);

        Ok(hash)
    }
}

impl<P: EthereumProvider + Send + Sync> DatabaseCommit for EthCacheDatabase<P> {
    fn commit(&mut self, changes: HashMap<Address, Account>) {
        for (address, account) in changes {
            let db_account = DbAccount {
                info: account.info.clone(),
                storage: account.storage.into_iter().map(|(k, v)| (k, v.present_value)).collect(),
                account_state: AccountState::None,
            };
            self.0.accounts.insert(address, db_account);
        }
    }
}
