use std::sync::Arc;

use crate::db_client::client::DbClient;
use crate::db_client::error::DatabaseError;
use crate::db_client::types::header::DbHeader;
use crate::db_client::types::transaction::{DbTransactionFull, DbTransactionHash};
use crate::into_via_wrapper;
use crate::models::block::EthBlockId;
use crate::models::event::StarknetEvent;
use crate::models::event_filter::EthEventFilter;
use crate::models::felt::Felt252Wrapper;
use crate::models::transaction::transaction::StarknetTransaction;
use crate::models::transaction_receipt::StarknetTransactionReceipt as TransactionReceiptWrapper;
use crate::starknet_client::constants::CHUNK_SIZE_LIMIT;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::helpers::try_from_u8_iterator;
use crate::starknet_client::{ContractAccountReader, KakarotClient};
use crate::{into_via_try_wrapper, into_via_wrapper};
use jsonrpsee::core::{async_trait, RpcResult as Result};
use mongodb::bson::doc;
use reth_primitives::{AccessListWithGasUsed, Address, BlockId, BlockNumberOrTag, Bytes, H256, H64, U128, U256, U64};
use reth_rpc_types::{
    Block, BlockTransactions, CallRequest, EIP1186AccountProofResponse, FeeHistory, Filter, FilterChanges, Index,
    RichBlock, SyncInfo, SyncStatus, Transaction as EtherTransaction, TransactionReceipt, TransactionRequest, Work,
};
use serde_json::Value;
use starknet::core::types::{
    BlockId as StarknetBlockId, Event, EventFilterWithPage, FieldElement, MaybePendingTransactionReceipt,
    ResultPageRequest, SyncStatusType, TransactionReceipt as StarknetTransactionReceipt,
};
use starknet::providers::Provider;

use crate::eth_rpc::api::eth_api::EthApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct KakarotEthRpc<P: Provider + Send + Sync + 'static> {
    pub kakarot_client: Arc<KakarotClient<P>>,
    pub db_client: DbClient,
}

impl<P: Provider + Send + Sync + 'static> KakarotEthRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>, db_client: DbClient) -> Self {
        Self { kakarot_client, db_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> EthApiServer for KakarotEthRpc<P> {
    #[tracing::instrument(skip_all, ret, err)]
    async fn block_number(&self) -> Result<U64> {
        let header = self.db_client.find_one::<DbHeader>("headers", doc! {}, doc! {"header.number": -1}).await?;
        let block_number = header.header.number.ok_or(DatabaseError::ValueNotFound)?.as_limbs()[0];
        Ok(U64::from(block_number))
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn syncing(&self) -> Result<SyncStatus> {
        let status = self.kakarot_client.starknet_provider().syncing().await.map_err(EthApiError::from)?;

        match status {
            SyncStatusType::NotSyncing => Ok(SyncStatus::None),

            SyncStatusType::Syncing(data) => {
                let starting_block: U256 = U256::from(data.starting_block_num);
                let current_block: U256 = U256::from(data.current_block_num);
                let highest_block: U256 = U256::from(data.highest_block_num);

                let status_info = SyncInfo {
                    starting_block,
                    current_block,
                    highest_block,
                    warp_chunks_amount: None,
                    warp_chunks_processed: None,
                };

                Ok(SyncStatus::Info(status_info))
            }
        }
    }

    async fn coinbase(&self) -> Result<Address> {
        Err(EthApiError::MethodNotSupported("eth_coinbase".to_string()).into())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn accounts(&self) -> Result<Vec<Address>> {
        Ok(Vec::new())
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn chain_id(&self) -> Result<Option<U64>> {
        let tx =
            self.db_client.find_one::<DbTransactionFull>("transactions", doc! {}, doc! {"tx.blockNumber": -1}).await?;
        Ok(tx.tx.chain_id)
    }

    #[tracing::instrument(skip_all, ret, err, fields(hash = %hash))]
    async fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>> {
        let header = self
            .db_client
            .find_one::<DbHeader>("headers", doc! {"header.hash": format!("0x{:064x}", hash)}, None)
            .await?;
        let total_difficulty = Some(header.header.difficulty);

        let transactions = if full {
            BlockTransactions::Full(
                self.db_client
                    .find_all::<DbTransactionFull>(
                        "transactions",
                        doc! {"tx.blockHash": format!("0x{:064x}", hash)},
                        None,
                    )
                    .await?
                    .into_iter()
                    .map(|tx| tx.tx)
                    .collect(),
            )
        } else {
            BlockTransactions::Hashes(
                self.db_client
                    .find_all::<DbTransactionHash>(
                        "transactions",
                        doc! {"tx.blockHash": format!("0x{:064x}", hash)},
                        doc! {"tx.blockHash": 1},
                    )
                    .await?
                    .into_iter()
                    .map(|tx| tx.tx.into())
                    .collect(),
            )
        };

        let block = Block {
            header: header.header,
            transactions,
            total_difficulty,
            uncles: Vec::new(),
            size: None,
            withdrawals: None,
        };

        Ok(Some(block.into()))
    }

    #[tracing::instrument(skip_all, ret, err, fields(number = %number, full = full))]
    async fn block_by_number(&self, number: BlockNumberOrTag, full: bool) -> Result<Option<RichBlock>> {
        let block_id = EthBlockId::new(BlockId::Number(number));
        let starknet_block_id: StarknetBlockId = block_id.try_into().map_err(EthApiError::from)?;
        let block = self.kakarot_client.get_eth_block_from_starknet_block(starknet_block_id, full).await?;
        Ok(Some(block))
    }

    #[tracing::instrument(skip_all, ret, err, fields(hash = %hash))]
    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64> {
        let block_id = BlockId::Hash(hash.into());
        let count = self.kakarot_client.get_transaction_count_by_block(block_id).await.map_err(EthApiError::from)?;
        Ok(count)
    }

    #[tracing::instrument(skip_all, ret, err, fields(number = %number))]
    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<U64> {
        let block_id = BlockId::Number(number);
        let count = self.kakarot_client.get_transaction_count_by_block(block_id).await.map_err(EthApiError::from)?;
        Ok(count)
    }

    async fn block_uncles_count_by_block_hash(&self, _hash: H256) -> Result<U256> {
        Err(EthApiError::MethodNotSupported("eth_getUncleCountByBlockHash".to_string()).into())
    }

    async fn block_uncles_count_by_block_number(&self, _number: BlockNumberOrTag) -> Result<U256> {
        Err(EthApiError::MethodNotSupported("eth_getUncleCountByBlockNumber".to_string()).into())
    }

    async fn uncle_by_block_hash_and_index(&self, _hash: H256, _index: Index) -> Result<Option<RichBlock>> {
        Err(EthApiError::MethodNotSupported("eth_getUncleByBlockHashAndIndex".to_string()).into())
    }

    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> Result<Option<RichBlock>> {
        Err(EthApiError::MethodNotSupported("eth_getUncleByBlockNumberAndIndex".to_string()).into())
    }

    #[tracing::instrument(skip_all, ret, err, fields(hash = %hash))]
    async fn transaction_by_hash(&self, hash: H256) -> Result<Option<EtherTransaction>> {
        let hash: Felt252Wrapper = hash.try_into().map_err(EthApiError::from)?;
        let hash: FieldElement = hash.into();

        let transaction: StarknetTransaction =
            match self.kakarot_client.starknet_provider().get_transaction_by_hash(hash).await {
                Err(_) => return Ok(None),
                Ok(transaction) => transaction.into(),
            };

        let tx_receipt = match self.kakarot_client.starknet_provider().get_transaction_receipt(hash).await {
            Err(_) => return Ok(None),
            Ok(receipt) => receipt,
        };

        let (block_hash, block_num) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => {
                (Some(into_via_wrapper!(tr.block_hash)), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };
        let eth_transaction = transaction.to_eth_transaction(&self.kakarot_client, block_hash, block_num, None).await?;
        Ok(Some(eth_transaction))
    }

    #[tracing::instrument(skip_all, ret, err, fields(hash = %hash, index = ?index))]
    async fn transaction_by_block_hash_and_index(&self, hash: H256, index: Index) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Hash(hash.into());
        let tx = self.kakarot_client.transaction_by_block_id_and_index(block_id, index).await?;
        Ok(Some(tx))
    }

    #[tracing::instrument(skip_all, ret, err, fields(number = %number, index = ?index))]
    async fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumberOrTag,
        index: Index,
    ) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Number(number);
        let tx = self.kakarot_client.transaction_by_block_id_and_index(block_id, index).await?;
        Ok(Some(tx))
    }

    #[tracing::instrument(skip_all, ret, err, fields(hash = %hash))]
    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>> {
        // TODO: Error when trying to transform 32 bytes hash to FieldElement
        let transaction_hash = into_via_try_wrapper!(hash);
        let starknet_tx_receipt: TransactionReceiptWrapper = match self
            .kakarot_client
            .starknet_provider()
            .get_transaction_receipt::<FieldElement>(transaction_hash)
            .await
        {
            Err(_) => return Ok(None),
            Ok(receipt) => receipt,
        }
        .into();

        let res_receipt = starknet_tx_receipt.to_eth_transaction_receipt(&self.kakarot_client).await?;
        Ok(res_receipt)
    }

    #[tracing::instrument(skip_all, ret, err, fields(address = %address, block_id = ?block_id))]
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let balance = self.kakarot_client.balance(address, block_id).await?;
        Ok(balance)
    }

    #[tracing::instrument(skip_all, ret, err, fields(address = %address, index = ?index, block_id = ?block_id))]
    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let value = self.kakarot_client.storage_at(address, index, block_id).await?;
        Ok(value)
    }

    #[tracing::instrument(skip_all, ret, err, fields(address = %address, block_id = ?block_id))]
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));

        let transaction_count = self.kakarot_client.nonce(address, block_id).await?;

        Ok(transaction_count)
    }

    #[tracing::instrument(skip_all, ret, err, fields(address = %address, block_id = ?block_id))]
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> Result<Bytes> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into().map_err(EthApiError::from)?;

        let starknet_contract_address = self.kakarot_client.compute_starknet_address(&address).await?;

        let provider = self.kakarot_client.starknet_provider();

        // Get the nonce of the contract account -> a storage variable
        let contract_account = ContractAccountReader::new(starknet_contract_address, provider);
        let (_, bytecode) =
            contract_account.bytecode().block_id(starknet_block_id).call().await.map_err(EthApiError::from)?;
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<u8>>(bytecode.0.into_iter())))
    }

    #[tracing::instrument(skip_all, ret, err, fields(filter = ?filter))]
    async fn get_logs(&self, filter: Filter) -> Result<FilterChanges> {
        // Check the block range
        let current_block: u64 =
            self.kakarot_client.starknet_provider().block_number().await.map_err(EthApiError::from)?;
        let from_block = filter.get_from_block();
        let to_block = filter.get_to_block();

        let filter = match (from_block, to_block) {
            (Some(from), _) if from > current_block => return Ok(FilterChanges::Empty),
            (_, Some(to)) if to > current_block => filter.to_block(current_block),
            (Some(from), Some(to)) if to < from => return Ok(FilterChanges::Empty),
            _ => filter,
        };

        // Convert the eth log filter to a starknet event filter
        let filter: EthEventFilter = filter.into();
        let event_filter = filter.to_starknet_event_filter(&self.kakarot_client.clone())?;

        // Filter events
        let events = self
            .kakarot_client
            .filter_events(EventFilterWithPage {
                event_filter,
                result_page_request: ResultPageRequest { continuation_token: None, chunk_size: CHUNK_SIZE_LIMIT },
            })
            .await?;

        // Convert events to eth logs
        let logs = events
            .into_iter()
            .filter_map(|emitted| {
                let event: StarknetEvent =
                    Event { from_address: emitted.from_address, keys: emitted.keys, data: emitted.data }.into();
                let block_hash = into_via_wrapper!(emitted.block_hash);
                let transaction_hash = into_via_wrapper!(emitted.transaction_hash);
                event
                    .to_eth_log(
                        &self.kakarot_client.clone(),
                        Some(block_hash),
                        Some(U256::from(emitted.block_number)),
                        Some(transaction_hash),
                        None,
                        None,
                    )
                    .ok()
            })
            .collect::<Vec<_>>();
        Ok(FilterChanges::Logs(logs))
    }

    #[tracing::instrument(skip_all, ret, err, fields(request = ?request, block_id = ?block_id))]
    async fn call(&self, request: CallRequest, block_id: Option<BlockId>) -> Result<Bytes> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let result = self.kakarot_client.call(request, block_id).await?;

        Ok(result)
    }

    async fn create_access_list(
        &self,
        _request: CallRequest,
        _block_id: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed> {
        Err(EthApiError::MethodNotSupported("eth_createAccessList".to_string()).into())
    }

    #[tracing::instrument(skip_all, ret, fields(request = ?request, block_id = ?block_id))]
    async fn estimate_gas(&self, request: CallRequest, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));

        Ok(self.kakarot_client.estimate_gas(request, block_id).await?)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn gas_price(&self) -> Result<U256> {
        let gas_price = self.kakarot_client.base_fee_per_gas();
        Ok(gas_price)
    }

    #[tracing::instrument(skip_all, ret, err, fields(block_count = %block_count, newest_block = %newest_block, reward_percentiles = ?reward_percentiles))]
    async fn fee_history(
        &self,
        block_count: U256,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory> {
        let fee_history = self.kakarot_client.fee_history(block_count, newest_block, reward_percentiles).await?;

        Ok(fee_history)
    }

    #[tracing::instrument(skip_all, ret, err)]
    async fn max_priority_fee_per_gas(&self) -> Result<U128> {
        let max_priority_fee = self.kakarot_client.max_priority_fee_per_gas();
        Ok(max_priority_fee)
    }

    async fn mining(&self) -> Result<bool> {
        Err(EthApiError::MethodNotSupported("eth_mining".to_string()).into())
    }

    async fn hashrate(&self) -> Result<U256> {
        Err(EthApiError::MethodNotSupported("eth_hashrate".to_string()).into())
    }

    async fn get_work(&self) -> Result<Work> {
        Err(EthApiError::MethodNotSupported("eth_getWork".to_string()).into())
    }

    async fn submit_hashrate(&self, _hashrate: U256, _id: H256) -> Result<bool> {
        Err(EthApiError::MethodNotSupported("eth_submitHashrate".to_string()).into())
    }

    async fn submit_work(&self, _nonce: H64, _pow_hash: H256, _mix_digest: H256) -> Result<bool> {
        Err(EthApiError::MethodNotSupported("eth_submitWork".to_string()).into())
    }

    async fn send_transaction(&self, _request: TransactionRequest) -> Result<H256> {
        Err(EthApiError::MethodNotSupported("eth_sendTransaction".to_string()).into())
    }

    #[tracing::instrument(skip_all, ret, err, fields(bytes = %bytes))]
    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256> {
        let transaction_hash = self.kakarot_client.send_transaction(bytes).await?;
        Ok(transaction_hash)
    }

    async fn sign(&self, _address: Address, _message: Bytes) -> Result<Bytes> {
        Err(EthApiError::MethodNotSupported("eth_sign".to_string()).into())
    }

    async fn sign_transaction(&self, _transaction: CallRequest) -> Result<Bytes> {
        Err(EthApiError::MethodNotSupported("eth_signTransaction".to_string()).into())
    }

    async fn sign_typed_data(&self, _address: Address, _data: Value) -> Result<Bytes> {
        Err(EthApiError::MethodNotSupported("eth_signTypedData".to_string()).into())
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<H256>,
        _block_id: Option<BlockId>,
    ) -> Result<EIP1186AccountProofResponse> {
        Err(EthApiError::MethodNotSupported("eth_getProof".to_string()).into())
    }

    async fn new_filter(&self, _filter: Filter) -> Result<U64> {
        Err(EthApiError::MethodNotSupported("eth_newFilter".to_string()).into())
    }

    async fn new_block_filter(&self) -> Result<U64> {
        Err(EthApiError::MethodNotSupported("eth_newBlockFilter".to_string()).into())
    }

    async fn new_pending_transaction_filter(&self) -> Result<U64> {
        Err(EthApiError::MethodNotSupported("eth_newPendingTransactionFilter".to_string()).into())
    }

    async fn uninstall_filter(&self, _id: U64) -> Result<bool> {
        Err(EthApiError::MethodNotSupported("eth_uninstallFilter".to_string()).into())
    }

    async fn get_filter_changes(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::MethodNotSupported("eth_getFilterChanges".to_string()).into())
    }

    async fn get_filter_logs(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::MethodNotSupported("eth_getFilterLogs".to_string()).into())
    }
}
