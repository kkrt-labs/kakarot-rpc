use crate::providers::eth_provider::{error::EthApiError, provider::EthereumProvider};
use reth_primitives::{Address, B256, U256};
use reth_revm::{
    db::CacheDB,
    primitives::{AccountInfo, Bytecode},
    DatabaseRef,
};
use reth_rpc_types::{serde_helpers::JsonStorageKey, BlockHashOrNumber, BlockId, BlockNumberOrTag};
use tokio::runtime::Handle;

#[derive(Debug, Clone)]
pub struct EthCacheDatabase<P: EthereumProvider + Send + Sync>(pub CacheDB<EthDatabase<P>>);

/// Ethereum database type.
#[derive(Debug, Clone)]
#[allow(clippy::redundant_pub_crate)]
pub struct EthDatabase<P: EthereumProvider + Send + Sync> {
    /// The Ethereum provider.
    provider: P,
    /// The block ID.
    block_id: BlockId,
}

impl<P: EthereumProvider + Send + Sync> EthDatabase<P> {
    pub const fn new(provider: P, block_id: BlockId) -> Self {
        Self { provider, block_id }
    }
}

/// The [`DatabaseRef`] trait implementation for [`EthDatabase`].
///
/// This implementation is designed to handle database interactions in a manner that is compatible
/// with both synchronous and asynchronous Rust contexts. Given the constraints of the underlying
/// database operations, it's necessary to perform blocking calls in a controlled manner to avoid
/// blocking the asynchronous runtime.
///
/// ### Why Use `tokio::task::block_in_place`?
///
/// The `tokio::task::block_in_place` function is employed here to enter a blocking context safely
/// within an asynchronous environment. This allows the blocking database operations to be executed
/// without hindering the performance of other asynchronous tasks or blocking the runtime.
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
