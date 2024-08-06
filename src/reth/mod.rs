use reth_evm::ConfigureEvm;
use reth_rpc::EthApi;
use reth_rpc_eth_types::{EthStateCache, FeeHistoryCache, FeeHistoryCacheConfig, GasPriceOracle};
use reth_rpc_server_types::constants::{DEFAULT_ETH_PROOF_WINDOW, DEFAULT_PROOF_PERMITS};
use reth_tasks::pool::BlockingTaskPool;

pub mod block;
pub mod evm;
pub mod receipt;
pub mod requests;
pub mod state;
pub mod transaction;
pub mod withdrawals;

#[derive(Debug)]
pub struct KakarotEthApi<Provider, Pool, Network, EvmConfig>(pub EthApi<Provider, Pool, Network, EvmConfig>);

impl<Pool, Network, EvmConfig> KakarotEthApi<KakarotProvider, Pool, Network, EvmConfig>
where
    Pool: Default,
    Network: Default,
    EvmConfig: Default + Clone + ConfigureEvm,
{
    pub fn new(pool: Pool, network: Network, evm_config: EvmConfig) -> Self {
        let provider = KakarotProvider {};
        let cache = EthStateCache::spawn(provider.clone(), Default::default(), evm_config.clone());
        let fee_history_cache = FeeHistoryCache::new(cache.clone(), FeeHistoryCacheConfig::default());

        let gas_cap = u64::MAX;
        Self(EthApi::new(
            provider.clone(),
            pool,
            network,
            cache.clone(),
            GasPriceOracle::new(provider, Default::default(), cache),
            gas_cap,
            DEFAULT_ETH_PROOF_WINDOW,
            BlockingTaskPool::build().expect("failed to build tracing pool"),
            fee_history_cache,
            evm_config,
            None,
            DEFAULT_PROOF_PERMITS,
        ))
    }
}

#[derive(Debug)]
pub struct KakarotProvider {}

impl Clone for KakarotProvider {
    fn clone(&self) -> Self {
        Self {}
    }
}

// impl BlockReaderIdExt for KakarotProvider {
//     fn block_by_id(&self, _id: BlockId) -> ProviderResult<Option<Block>> {
//         Ok(None)
//     }

//     fn sealed_header_by_id(&self, _id: BlockId) -> ProviderResult<Option<SealedHeader>> {
//         Ok(None)
//     }

//     fn header_by_id(&self, _id: BlockId) -> ProviderResult<Option<Header>> {
//         Ok(None)
//     }

//     fn ommers_by_id(&self, _id: BlockId) -> ProviderResult<Option<Vec<Header>>> {
//         Ok(None)
//     }
// }

// impl ReceiptProvider for KakarotProvider {
//     fn receipt(&self, _id: TxNumber) -> ProviderResult<Option<Receipt>> {
//         Ok(None)
//     }

//     fn receipt_by_hash(&self, _hash: TxHash) -> ProviderResult<Option<Receipt>> {
//         Ok(None)
//     }

//     fn receipts_by_block(&self, _block: BlockHashOrNumber) -> ProviderResult<Option<Vec<Receipt>>> {
//         Ok(None)
//     }

//     fn receipts_by_tx_range(&self, _range: impl RangeBounds<TxNumber>) -> ProviderResult<Vec<Receipt>> {
//         Ok(vec![])
//     }
// }

// impl BlockIdReader for KakarotProvider {
//     fn pending_block_num_hash(&self) -> ProviderResult<Option<BlockNumHash>> {
//         Ok(None)
//     }

//     fn safe_block_num_hash(&self) -> ProviderResult<Option<BlockNumHash>> {
//         Ok(None)
//     }

//     fn finalized_block_num_hash(&self) -> ProviderResult<Option<BlockNumHash>> {
//         Ok(None)
//     }
// }

// impl BlockNumReader for KakarotProvider {
//     fn chain_info(&self) -> ProviderResult<ChainInfo> {
//         Ok(Default::default())
//     }

//     fn best_block_number(&self) -> ProviderResult<BlockNumber> {
//         Ok(0)
//     }

//     fn last_block_number(&self) -> ProviderResult<BlockNumber> {
//         Ok(0)
//     }

//     fn block_number(&self, _hash: B256) -> ProviderResult<Option<BlockNumber>> {
//         Ok(None)
//     }
// }

// impl BlockHashReader for KakarotProvider {
//     fn block_hash(&self, _number: BlockNumber) -> ProviderResult<Option<B256>> {
//         Ok(None)
//     }

//     fn canonical_hashes_range(&self, _start: BlockNumber, _end: BlockNumber) -> ProviderResult<Vec<B256>> {
//         Ok(vec![])
//     }
// }

// impl ReceiptProviderIdExt for KakarotProvider {}

// impl HeaderProvider for KakarotProvider {
//     fn header(&self, _block_hash: &BlockHash) -> ProviderResult<Option<Header>> {
//         Ok(None)
//     }

//     fn header_by_number(&self, _num: u64) -> ProviderResult<Option<Header>> {
//         Ok(None)
//     }

//     fn header_by_hash_or_number(&self, _hash_or_num: BlockHashOrNumber) -> ProviderResult<Option<Header>> {
//         Ok(None)
//     }

//     fn header_td(&self, _hash: &BlockHash) -> ProviderResult<Option<U256>> {
//         Ok(None)
//     }

//     fn header_td_by_number(&self, _number: BlockNumber) -> ProviderResult<Option<U256>> {
//         Ok(None)
//     }

//     fn headers_range(&self, _range: impl RangeBounds<BlockNumber>) -> ProviderResult<Vec<Header>> {
//         Ok(vec![])
//     }

//     fn sealed_header(&self, _number: BlockNumber) -> ProviderResult<Option<reth_primitives::SealedHeader>> {
//         Ok(None)
//     }

//     fn sealed_headers_range(&self, _range: impl RangeBounds<BlockNumber>) -> ProviderResult<Vec<SealedHeader>> {
//         Ok(vec![])
//     }

//     fn sealed_headers_while(
//         &self,
//         _range: impl RangeBounds<BlockNumber>,
//         _predicate: impl FnMut(&SealedHeader) -> bool,
//     ) -> ProviderResult<Vec<SealedHeader>> {
//         Ok(vec![])
//     }
// }

// impl TransactionsProvider for KakarotProvider {
//     fn transaction_id(&self, _tx_hash: TxHash) -> ProviderResult<Option<TxNumber>> {
//         Ok(None)
//     }

//     fn transaction_by_id(&self, _id: TxNumber) -> ProviderResult<Option<TransactionSigned>> {
//         Ok(None)
//     }

//     fn transaction_by_id_no_hash(&self, _id: TxNumber) -> ProviderResult<Option<TransactionSignedNoHash>> {
//         Ok(None)
//     }

//     fn transaction_by_hash(&self, _hash: TxHash) -> ProviderResult<Option<TransactionSigned>> {
//         Ok(None)
//     }

//     fn transaction_by_hash_with_meta(
//         &self,
//         _hash: TxHash,
//     ) -> ProviderResult<Option<(TransactionSigned, TransactionMeta)>> {
//         Ok(None)
//     }

//     fn transaction_block(&self, _id: TxNumber) -> ProviderResult<Option<BlockNumber>> {
//         Ok(None)
//     }

//     fn transactions_by_block(&self, _block: BlockHashOrNumber) -> ProviderResult<Option<Vec<TransactionSigned>>> {
//         Ok(None)
//     }

//     fn transactions_by_block_range(
//         &self,
//         _range: impl RangeBounds<BlockNumber>,
//     ) -> ProviderResult<Vec<Vec<TransactionSigned>>> {
//         Ok(vec![])
//     }

//     fn transactions_by_tx_range(
//         &self,
//         _range: impl RangeBounds<TxNumber>,
//     ) -> ProviderResult<Vec<TransactionSignedNoHash>> {
//         Ok(vec![])
//     }

//     fn senders_by_tx_range(&self, _range: impl RangeBounds<TxNumber>) -> ProviderResult<Vec<Address>> {
//         Ok(vec![])
//     }

//     fn transaction_sender(&self, _id: TxNumber) -> ProviderResult<Option<Address>> {
//         Ok(None)
//     }
// }

// impl RequestsProvider for KakarotProvider {
//     fn requests_by_block(&self, _id: BlockHashOrNumber, _timestamp: u64) -> ProviderResult<Option<Requests>> {
//         Ok(None)
//     }
// }

// impl WithdrawalsProvider for KakarotProvider {
//     fn withdrawals_by_block(
//         &self,
//         _id: BlockHashOrNumber,
//         _timestamp: u64,
//     ) -> ProviderResult<Option<reth_primitives::Withdrawals>> {
//         Ok(None)
//     }

//     fn latest_withdrawal(&self) -> ProviderResult<Option<reth_primitives::Withdrawal>> {
//         Ok(None)
//     }
// }

// impl BlockReader for KakarotProvider {
//     fn find_block_by_hash(&self, _hash: B256, _source: BlockSource) -> ProviderResult<Option<Block>> {
//         Ok(None)
//     }

//     fn block(&self, _id: BlockHashOrNumber) -> ProviderResult<Option<Block>> {
//         Ok(None)
//     }

//     fn pending_block(&self) -> ProviderResult<Option<SealedBlock>> {
//         Ok(None)
//     }

//     fn pending_block_with_senders(&self) -> ProviderResult<Option<SealedBlockWithSenders>> {
//         Ok(None)
//     }

//     fn pending_block_and_receipts(&self) -> ProviderResult<Option<(SealedBlock, Vec<Receipt>)>> {
//         Ok(None)
//     }

//     fn ommers(&self, _id: BlockHashOrNumber) -> ProviderResult<Option<Vec<Header>>> {
//         Ok(None)
//     }

//     fn block_body_indices(&self, _num: u64) -> ProviderResult<Option<StoredBlockBodyIndices>> {
//         Ok(None)
//     }

//     fn block_with_senders(
//         &self,
//         _id: BlockHashOrNumber,
//         _transaction_kind: TransactionVariant,
//     ) -> ProviderResult<Option<BlockWithSenders>> {
//         Ok(None)
//     }

//     fn sealed_block_with_senders(
//         &self,
//         _id: BlockHashOrNumber,
//         _transaction_kind: TransactionVariant,
//     ) -> ProviderResult<Option<SealedBlockWithSenders>> {
//         Ok(None)
//     }

//     fn block_range(&self, _range: RangeInclusive<BlockNumber>) -> ProviderResult<Vec<Block>> {
//         Ok(vec![])
//     }

//     fn block_with_senders_range(&self, _range: RangeInclusive<BlockNumber>) -> ProviderResult<Vec<BlockWithSenders>> {
//         Ok(vec![])
//     }

//     fn sealed_block_with_senders_range(
//         &self,
//         _range: RangeInclusive<BlockNumber>,
//     ) -> ProviderResult<Vec<SealedBlockWithSenders>> {
//         Ok(vec![])
//     }
// }

// impl StateProofProvider for KakarotProvider {
//     fn proof(&self, _state: &BundleState, _address: Address, _slots: &[B256]) -> ProviderResult<AccountProof> {
//         Ok(AccountProof::new(Address::default()))
//     }
// }

// impl StateRootProvider for KakarotProvider {
//     fn state_root(&self, _bundle_state: &BundleState) -> ProviderResult<B256> {
//         Ok(B256::default())
//     }

//     fn state_root_with_updates(&self, _bundle_state: &BundleState) -> ProviderResult<(B256, TrieUpdates)> {
//         Ok((B256::default(), TrieUpdates::default()))
//     }
// }

// impl AccountReader for KakarotProvider {
//     fn basic_account(&self, _address: Address) -> ProviderResult<Option<Account>> {
//         Ok(None)
//     }
// }

// impl StateProvider for KakarotProvider {
//     fn storage(&self, _account: Address, _storage_key: StorageKey) -> ProviderResult<Option<StorageValue>> {
//         Ok(None)
//     }

//     fn bytecode_by_hash(&self, _code_hash: B256) -> ProviderResult<Option<Bytecode>> {
//         Ok(None)
//     }
// }

// impl EvmEnvProvider for KakarotProvider {
//     fn fill_env_at<EvmConfig>(
//         &self,
//         _cfg: &mut CfgEnvWithHandlerCfg,
//         _block_env: &mut BlockEnv,
//         _at: BlockHashOrNumber,
//         _evm_config: EvmConfig,
//     ) -> ProviderResult<()>
//     where
//         EvmConfig: ConfigureEvmEnv,
//     {
//         Ok(())
//     }

//     fn fill_env_with_header<EvmConfig>(
//         &self,
//         _cfg: &mut CfgEnvWithHandlerCfg,
//         _block_env: &mut BlockEnv,
//         _header: &Header,
//         _evm_config: EvmConfig,
//     ) -> ProviderResult<()>
//     where
//         EvmConfig: ConfigureEvmEnv,
//     {
//         Ok(())
//     }

//     fn fill_cfg_env_at<EvmConfig>(
//         &self,
//         _cfg: &mut CfgEnvWithHandlerCfg,
//         _at: BlockHashOrNumber,
//         _evm_config: EvmConfig,
//     ) -> ProviderResult<()>
//     where
//         EvmConfig: ConfigureEvmEnv,
//     {
//         Ok(())
//     }

//     fn fill_cfg_env_with_header<EvmConfig>(
//         &self,
//         _cfg: &mut CfgEnvWithHandlerCfg,
//         _header: &Header,
//         _evm_config: EvmConfig,
//     ) -> ProviderResult<()>
//     where
//         EvmConfig: ConfigureEvmEnv,
//     {
//         Ok(())
//     }
// }

// impl StateProviderFactory for KakarotProvider {
//     fn latest(&self) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }

//     fn history_by_block_number(&self, _block: BlockNumber) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }

//     fn history_by_block_hash(&self, _block: BlockHash) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }

//     fn state_by_block_hash(&self, _block: BlockHash) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }

//     fn pending(&self) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }

//     fn pending_state_by_hash(&self, _block_hash: B256) -> ProviderResult<Option<StateProviderBox>> {
//         Ok(Some(Box::new(self.clone())))
//     }

//     fn pending_with_provider(
//         &self,
//         _bundle_state_data: Box<dyn reth_storage_api::FullExecutionDataProvider>,
//     ) -> ProviderResult<StateProviderBox> {
//         Ok(Box::new(self.clone()))
//     }
// }
