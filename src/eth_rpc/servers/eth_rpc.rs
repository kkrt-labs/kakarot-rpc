#![allow(clippy::blocks_in_conditions)]

use jsonrpsee::core::{async_trait, RpcResult as Result};
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, B256, B64, U256, U64};
use reth_rpc_types::serde_helpers::JsonStorageKey;
use reth_rpc_types::{
    AccessListWithGasUsed, EIP1186AccountProofResponse, FeeHistory, Filter, FilterChanges, Index, RichBlock,
    SyncStatus, Transaction, TransactionReceipt, TransactionRequest, Work,
};
use serde_json::Value;

use crate::eth_provider::constant::MAX_PRIORITY_FEE_PER_GAS;
use crate::eth_provider::error::EthApiError;
use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::api::eth_api::EthApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
#[derive(Debug)]
pub struct KakarotEthRpc<P>
where
    P: EthereumProvider,
{
    eth_provider: P,
}

impl<P> KakarotEthRpc<P>
where
    P: EthereumProvider,
{
    pub const fn new(eth_provider: P) -> Self {
        Self { eth_provider }
    }
}

#[async_trait]
impl<P> EthApiServer for KakarotEthRpc<P>
where
    P: EthereumProvider + Send + Sync + 'static,
{
    #[tracing::instrument(skip_all, ret, err)]
    async fn block_number(&self) -> Result<U64> {
        tracing::info!("Serving eth_blockNumber");
        Ok(self.eth_provider.block_number().await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn syncing(&self) -> Result<SyncStatus> {
        tracing::info!("Serving eth_syncing");
        Ok(self.eth_provider.syncing().await?)
    }

    async fn coinbase(&self) -> Result<Address> {
        Err(EthApiError::Unsupported("eth_coinbase").into())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn accounts(&self) -> Result<Vec<Address>> {
        tracing::info!("Serving eth_accounts");
        Ok(Vec::new())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn chain_id(&self) -> Result<Option<U64>> {
        tracing::info!("Serving eth_chainId");
        Ok(self.eth_provider.chain_id().await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %hash))]
    async fn block_by_hash(&self, hash: B256, full: bool) -> Result<Option<RichBlock>> {
        tracing::info!("Serving eth_getBlockByHash");
        Ok(self.eth_provider.block_by_hash(hash, full).await?)
    }

    #[tracing::instrument(skip(self), err, fields(number = %number, full = full))]
    async fn block_by_number(&self, number: BlockNumberOrTag, full: bool) -> Result<Option<RichBlock>> {
        tracing::info!("Serving eth_getBlockByNumber");
        Ok(self.eth_provider.block_by_number(number, full).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %hash))]
    async fn block_transaction_count_by_hash(&self, hash: B256) -> Result<Option<U256>> {
        tracing::info!("Serving eth_getBlockTransactionCountByHash");
        Ok(self.eth_provider.block_transaction_count_by_hash(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(number = %number))]
    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<Option<U256>> {
        tracing::info!("Serving eth_getBlockTransactionCountByNumber");
        Ok(self.eth_provider.block_transaction_count_by_number(number).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %_hash))]
    async fn block_uncles_count_by_block_hash(&self, _hash: B256) -> Result<U256> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(U256::ZERO)
    }

    #[tracing::instrument(skip(self), ret, err, fields(number = %_number))]
    async fn block_uncles_count_by_block_number(&self, _number: BlockNumberOrTag) -> Result<U256> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(U256::ZERO)
    }

    #[tracing::instrument(skip(self), err, fields(hash = %_hash, index = ?_index))]
    async fn uncle_by_block_hash_and_index(&self, _hash: B256, _index: Index) -> Result<Option<RichBlock>> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(None)
    }

    #[tracing::instrument(skip(self), err, fields(hash = %_number, index = ?_index))]
    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> Result<Option<RichBlock>> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(None)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %hash))]
    async fn transaction_by_hash(&self, hash: B256) -> Result<Option<Transaction>> {
        tracing::info!("Serving eth_getTransactionByHash");
        Ok(self.eth_provider.transaction_by_hash(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %hash, index = ?index))]
    async fn transaction_by_block_hash_and_index(&self, hash: B256, index: Index) -> Result<Option<Transaction>> {
        tracing::info!("Serving eth_getTransactionByBlockHashAndIndex");
        Ok(self.eth_provider.transaction_by_block_hash_and_index(hash, index).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(number = %number, index = ?index))]
    async fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumberOrTag,
        index: Index,
    ) -> Result<Option<Transaction>> {
        tracing::info!("Serving eth_getTransactionByBlockNumberAndIndex");
        Ok(self.eth_provider.transaction_by_block_number_and_index(number, index).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(hash = %hash))]
    async fn transaction_receipt(&self, hash: B256) -> Result<Option<TransactionReceipt>> {
        tracing::info!("Serving eth_getTransactionReceipt");
        Ok(self.eth_provider.transaction_receipt(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(address = %address, block_id = ?block_id))]
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        tracing::info!("Serving eth_getBalance");
        Ok(self.eth_provider.balance(address, block_id).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(address = %address, index = ?index, block_id = ?block_id))]
    async fn storage_at(&self, address: Address, index: JsonStorageKey, block_id: Option<BlockId>) -> Result<B256> {
        tracing::info!("Serving eth_getStorageAt");
        Ok(self.eth_provider.storage_at(address, index, block_id).await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(address = %address, block_id = ?block_id))]
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        tracing::info!("Serving eth_getTransactionCount");
        Ok(self.eth_provider.transaction_count(address, block_id).await?)
    }

    #[tracing::instrument(skip(self), err, fields(address = %address, block_id = ?block_id))]
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> Result<Bytes> {
        tracing::info!("Serving eth_getCode");
        Ok(self.eth_provider.get_code(address, block_id).await?)
    }

    #[tracing::instrument(skip(self), err, fields(filter = ?filter))]
    async fn get_logs(&self, filter: Filter) -> Result<FilterChanges> {
        tracing::info!("Serving eth_getLogs");
        Ok(self.eth_provider.get_logs(filter).await?)
    }

    #[tracing::instrument(skip(self, request), err, fields(block_id = ?block_id, gas_limit = request.gas))]
    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>) -> Result<Bytes> {
        tracing::info!("Serving eth_call");
        Ok(self.eth_provider.call(request, block_id).await?)
    }

    async fn create_access_list(
        &self,
        _request: TransactionRequest,
        _block_id: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed> {
        Err(EthApiError::Unsupported("eth_createAccessList").into())
    }

    #[tracing::instrument(skip(self, request), err, fields(block_id = ?block_id, gas_limit = request.gas))]
    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> Result<U256> {
        tracing::info!("Serving eth_estimateGas");
        Ok(self.eth_provider.estimate_gas(request, block_id).await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn gas_price(&self) -> Result<U256> {
        tracing::info!("Serving eth_gasPrice");
        Ok(self.eth_provider.gas_price().await?)
    }

    #[tracing::instrument(skip(self), ret, err, fields(block_count = ?block_count, newest_block = %newest_block, reward_percentiles = ?reward_percentiles))]
    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory> {
        tracing::info!("Serving eth_feeHistory");
        Ok(self.eth_provider.fee_history(block_count, newest_block, reward_percentiles).await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn max_priority_fee_per_gas(&self) -> Result<U256> {
        tracing::info!("Serving eth_maxPriorityFeePerGas");
        Ok(U256::from(*MAX_PRIORITY_FEE_PER_GAS))
    }

    async fn blob_base_fee(&self) -> Result<U256> {
        Err(EthApiError::Unsupported("eth_blobBaseFee").into())
    }

    async fn mining(&self) -> Result<bool> {
        tracing::warn!("Kakarot chain does not use mining");
        Ok(false)
    }

    async fn hashrate(&self) -> Result<U256> {
        tracing::warn!("Kakarot chain does not produce hash rate");
        Ok(U256::ZERO)
    }

    async fn get_work(&self) -> Result<Work> {
        tracing::warn!("Kakarot chain does not produce work");
        Ok(Work::default())
    }

    async fn submit_hashrate(&self, _hashrate: U256, _id: B256) -> Result<bool> {
        Err(EthApiError::Unsupported("eth_submitHashrate").into())
    }

    async fn submit_work(&self, _nonce: B64, _pow_hash: B256, _mix_digest: B256) -> Result<bool> {
        Err(EthApiError::Unsupported("eth_submitWork").into())
    }

    async fn send_transaction(&self, _request: TransactionRequest) -> Result<B256> {
        Err(EthApiError::Unsupported("eth_sendTransaction").into())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<B256> {
        tracing::info!("Serving eth_sendRawTransaction");
        Ok(self.eth_provider.send_raw_transaction(bytes).await?)
    }

    async fn sign(&self, _address: Address, _message: Bytes) -> Result<Bytes> {
        Err(EthApiError::Unsupported("eth_sign").into())
    }

    async fn sign_transaction(&self, _transaction: TransactionRequest) -> Result<Bytes> {
        Err(EthApiError::Unsupported("eth_signTransaction").into())
    }

    async fn sign_typed_data(&self, _address: Address, _data: Value) -> Result<Bytes> {
        Err(EthApiError::Unsupported("eth_signTypedData").into())
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<B256>,
        _block_id: Option<BlockId>,
    ) -> Result<EIP1186AccountProofResponse> {
        Err(EthApiError::Unsupported("eth_getProof").into())
    }

    async fn new_filter(&self, _filter: Filter) -> Result<U64> {
        Err(EthApiError::Unsupported("eth_newFilter").into())
    }

    async fn new_block_filter(&self) -> Result<U64> {
        Err(EthApiError::Unsupported("eth_newBlockFilter").into())
    }

    async fn new_pending_transaction_filter(&self) -> Result<U64> {
        Err(EthApiError::Unsupported("eth_newPendingTransactionFilter").into())
    }

    async fn uninstall_filter(&self, _id: U64) -> Result<bool> {
        Err(EthApiError::Unsupported("eth_uninstallFilter").into())
    }

    async fn get_filter_changes(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::Unsupported("eth_getFilterChanges").into())
    }

    async fn get_filter_logs(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::Unsupported("eth_getFilterLogs").into())
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> Result<Option<Vec<TransactionReceipt>>> {
        Ok(self.eth_provider.block_receipts(block_id).await?)
    }
}
