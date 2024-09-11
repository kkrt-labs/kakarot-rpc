use crate::{
    client::{EthClient, KakarotTransactions, TransactionHashProvider},
    eth_rpc::api::eth_api::EthApiServer,
    providers::eth_provider::{
        constant::{MAX_PRIORITY_FEE_PER_GAS, MAIN_RPC_URL},
        database::types::{header::ExtendedBlock, receipt::ExtendedTxReceipt, transaction::ExtendedTransaction},
        error::EthApiError,
        BlockProvider, ChainProvider, GasProvider, LogProvider, ReceiptProvider, StateProvider, TransactionProvider,
    },
};
use alloy_primitives::{Address, Bytes, B256, B64, U256, U64};
use alloy_rpc_types::{
    serde_helpers::JsonStorageKey, state::StateOverride, AccessListResult, BlockOverrides, EIP1186AccountProofResponse,
    FeeHistory, Filter, FilterChanges, Index, SyncStatus, TransactionRequest, Work,
};
use jsonrpsee::core::{async_trait, RpcResult};
use reth_primitives::{BlockId, BlockNumberOrTag};
use serde_json::Value;
use starknet::providers::Provider;
use std::sync::Arc;
use url::Url;

/// The RPC module for the Ethereum protocol required by Kakarot.
#[derive(Debug)]
pub struct EthRpc<SP>
where
    SP: Provider + Send + Sync,
{
    eth_client: Arc<EthClient<SP>>,
}

impl<SP> EthRpc<SP>
where
    SP: Provider + Send + Sync,
{
    pub const fn new(eth_client: Arc<EthClient<SP>>) -> Self {
        Self { eth_client }
    }
}

#[async_trait]
impl<SP> EthApiServer for EthRpc<SP>
where
    SP: Provider + Clone + Send + Sync + 'static,
{
    #[tracing::instrument(skip_all, ret, err)]
    async fn block_number(&self) -> RpcResult<U64> {
        Ok(self.eth_client.eth_provider().block_number().await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn syncing(&self) -> RpcResult<SyncStatus> {
        Ok(self.eth_client.eth_provider().syncing().await?)
    }

    async fn coinbase(&self) -> RpcResult<Address> {
        Err(EthApiError::Unsupported("eth_coinbase").into())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn accounts(&self) -> RpcResult<Vec<Address>> {
        Ok(Vec::new())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn chain_id(&self) -> RpcResult<Option<U64>> {
        Ok(self.eth_client.eth_provider().chain_id().await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn block_by_hash(&self, hash: B256, full: bool) -> RpcResult<Option<ExtendedBlock>> {
        Ok(self.eth_client.eth_provider().block_by_hash(hash, full).await?)
    }

    #[tracing::instrument(skip(self), err)]
    async fn block_by_number(&self, number: BlockNumberOrTag, full: bool) -> RpcResult<Option<ExtendedBlock>> {
        Ok(self.eth_client.eth_provider().block_by_number(number, full).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn block_transaction_count_by_hash(&self, hash: B256) -> RpcResult<Option<U256>> {
        Ok(self.eth_client.eth_provider().block_transaction_count_by_hash(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> RpcResult<Option<U256>> {
        Ok(self.eth_client.eth_provider().block_transaction_count_by_number(number).await?)
    }

    async fn block_uncles_count_by_block_hash(&self, _hash: B256) -> RpcResult<U256> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(U256::ZERO)
    }

    async fn block_uncles_count_by_block_number(&self, _number: BlockNumberOrTag) -> RpcResult<U256> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(U256::ZERO)
    }

    async fn uncle_by_block_hash_and_index(&self, _hash: B256, _index: Index) -> RpcResult<Option<ExtendedBlock>> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(None)
    }

    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> RpcResult<Option<ExtendedBlock>> {
        tracing::warn!("Kakarot chain does not produce uncles");
        Ok(None)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn transaction_by_hash(&self, hash: B256) -> RpcResult<Option<ExtendedTransaction>> {
        Ok(self.eth_client.transaction_by_hash(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> RpcResult<Option<ExtendedTransaction>> {
        Ok(self.eth_client.eth_provider().transaction_by_block_hash_and_index(hash, index).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumberOrTag,
        index: Index,
    ) -> RpcResult<Option<ExtendedTransaction>> {
        Ok(self.eth_client.eth_provider().transaction_by_block_number_and_index(number, index).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn transaction_receipt(&self, hash: B256) -> RpcResult<Option<ExtendedTxReceipt>> {
        Ok(self.eth_client.eth_provider().transaction_receipt(hash).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> RpcResult<U256> {
        Ok(self.eth_client.eth_provider().balance(address, block_id).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn storage_at(&self, address: Address, index: JsonStorageKey, block_id: Option<BlockId>) -> RpcResult<B256> {
        Ok(self.eth_client.eth_provider().storage_at(address, index, block_id).await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> RpcResult<U256> {
        Ok(self.eth_client.eth_provider().transaction_count(address, block_id).await?)
    }

    #[tracing::instrument(skip(self), err)]
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> RpcResult<Bytes> {
        Ok(self.eth_client.eth_provider().get_code(address, block_id).await?)
    }

    #[tracing::instrument(skip_all, err)]
    async fn get_logs(&self, filter: Filter) -> RpcResult<FilterChanges> {
        tracing::info!(?filter);
        Ok(self.eth_client.eth_provider().get_logs(filter).await?)
    }

    #[tracing::instrument(skip(self, request), err)]
    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> RpcResult<Bytes> {
        Ok(self.eth_client.eth_provider().call(request, block_id, state_overrides, block_overrides).await?)
    }

    async fn create_access_list(
        &self,
        _request: TransactionRequest,
        _block_id: Option<BlockId>,
    ) -> RpcResult<AccessListResult> {
        Err(EthApiError::Unsupported("eth_createAccessList").into())
    }

    #[tracing::instrument(skip(self, request), err)]
    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> RpcResult<U256> {
        Ok(U256::from(self.eth_client.eth_provider().estimate_gas(request, block_id).await?))
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn gas_price(&self) -> RpcResult<U256> {
        Ok(self.eth_client.eth_provider().gas_price().await?)
    }

    #[tracing::instrument(skip(self), ret, err)]
    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> RpcResult<FeeHistory> {
        tracing::info!("Serving eth_feeHistory");
        Ok(self.eth_client.eth_provider().fee_history(block_count, newest_block, reward_percentiles).await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn max_priority_fee_per_gas(&self) -> RpcResult<U256> {
        Ok(U256::from(*MAX_PRIORITY_FEE_PER_GAS))
    }

    async fn blob_base_fee(&self) -> RpcResult<U256> {
        Err(EthApiError::Unsupported("eth_blobBaseFee").into())
    }

    async fn mining(&self) -> RpcResult<bool> {
        tracing::warn!("Kakarot chain does not use mining");
        Ok(false)
    }

    async fn hashrate(&self) -> RpcResult<U256> {
        tracing::warn!("Kakarot chain does not produce hash rate");
        Ok(U256::ZERO)
    }

    async fn get_work(&self) -> RpcResult<Work> {
        tracing::warn!("Kakarot chain does not produce work");
        Ok(Work::default())
    }

    async fn submit_hashrate(&self, _hashrate: U256, _id: B256) -> RpcResult<bool> {
        Err(EthApiError::Unsupported("eth_submitHashrate").into())
    }

    async fn submit_work(&self, _nonce: B64, _pow_hash: B256, _mix_digest: B256) -> RpcResult<bool> {
        Err(EthApiError::Unsupported("eth_submitWork").into())
    }

    async fn send_transaction(&self, _request: TransactionRequest) -> RpcResult<B256> {
        Err(EthApiError::Unsupported("eth_sendTransaction").into())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn send_raw_transaction(&self, bytes: Bytes) -> RpcResult<B256> {
        tracing::info!("Serving eth_sendRawTransaction");
        #[cfg(feature = "rpc_forwarding")]
        {
            let provider_builded = ProviderBuilder::new().on_http(Url::parse(&MAIN_RPC_URL).expect("invalid rpc url"));

            let tx_hash = provider_builded
                .send_raw_transaction(&bytes)
                .await
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?;

            return Ok(*tx_hash.tx_hash());
        }
        Ok(self.eth_client.send_raw_transaction(bytes).await?)
    }

    async fn sign(&self, _address: Address, _message: Bytes) -> RpcResult<Bytes> {
        Err(EthApiError::Unsupported("eth_sign").into())
    }

    async fn sign_transaction(&self, _transaction: TransactionRequest) -> RpcResult<Bytes> {
        Err(EthApiError::Unsupported("eth_signTransaction").into())
    }

    async fn sign_typed_data(&self, _address: Address, _data: Value) -> RpcResult<Bytes> {
        Err(EthApiError::Unsupported("eth_signTypedData").into())
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<B256>,
        _block_id: Option<BlockId>,
    ) -> RpcResult<EIP1186AccountProofResponse> {
        Err(EthApiError::Unsupported("eth_getProof").into())
    }

    async fn new_filter(&self, _filter: Filter) -> RpcResult<U64> {
        Err(EthApiError::Unsupported("eth_newFilter").into())
    }

    async fn new_block_filter(&self) -> RpcResult<U64> {
        Err(EthApiError::Unsupported("eth_newBlockFilter").into())
    }

    async fn new_pending_transaction_filter(&self) -> RpcResult<U64> {
        Err(EthApiError::Unsupported("eth_newPendingTransactionFilter").into())
    }

    async fn uninstall_filter(&self, _id: U64) -> RpcResult<bool> {
        Err(EthApiError::Unsupported("eth_uninstallFilter").into())
    }

    async fn get_filter_changes(&self, _id: U64) -> RpcResult<FilterChanges> {
        Err(EthApiError::Unsupported("eth_getFilterChanges").into())
    }

    async fn get_filter_logs(&self, _id: U64) -> RpcResult<FilterChanges> {
        Err(EthApiError::Unsupported("eth_getFilterLogs").into())
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> RpcResult<Option<Vec<ExtendedTxReceipt>>> {
        Ok(self.eth_client.eth_provider().block_receipts(block_id).await?)
    }
}