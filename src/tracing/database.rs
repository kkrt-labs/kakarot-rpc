use crate::eth_provider::{
    error::EthApiError,
    provider::{EthDataProvider, EthereumProvider},
};
use reth_primitives::{Address, B256, U256};
use reth_rpc_types::BlockId;
use revm_core::{
    db::CacheDB,
    primitives::{AccountInfo, Bytecode},
    Database,
};
use tokio::runtime::Handle;

pub struct EthDatabaseSnapshot<SP: starknet::providers::Provider> {
    cache: CacheDB<EthDataProvider<SP>>,
    block_id: BlockId,
}

impl<SP: starknet::providers::Provider> EthDatabaseSnapshot<SP> {
    pub fn new(provider: EthDataProvider<SP>, block_id: BlockId) -> Self {
        Self { cache: CacheDB::new(provider), block_id }
    }
}

impl<SP> Database for EthDatabaseSnapshot<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    type Error = EthApiError;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Self::Error> {
        let cache = &mut self.cache;

        tokio::task::block_in_place(|| {
            Handle::current().block_on(async {
                let code = cache.db.get_code(address, Some(self.block_id.clone())).await;
                Ok(account)
            })
        });
        Ok(None)
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Self::Error> {
        todo!()
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Self::Error> {
        todo!()
    }

    fn block_hash(&mut self, number: U256) -> Result<B256, Self::Error> {
        todo!()
    }
}
