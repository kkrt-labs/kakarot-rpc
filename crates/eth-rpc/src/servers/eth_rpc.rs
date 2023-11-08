use std::sync::Arc;

use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::config::Network;
use kakarot_rpc_core::client::constants::gas::{BASE_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS, MINIMUM_GAS_FEE};
use kakarot_rpc_core::client::constants::{
    CHAIN_ID, CHUNK_SIZE_LIMIT, ESTIMATE_GAS, MAX_FEE, STARKNET_NATIVE_TOKEN, TX_ORIGIN_ZERO,
};
use kakarot_rpc_core::client::errors::{rpc_err, EthApiError, EthRpcErrorCode};
use kakarot_rpc_core::client::helpers::{bytes_to_felt_vec, raw_kakarot_calldata, DataDecodingError};
use kakarot_rpc_core::client::KakarotClient;
use kakarot_rpc_core::contracts::account::Account;
use kakarot_rpc_core::contracts::contract_account::ContractAccount;
use kakarot_rpc_core::contracts::erc20::starknet_erc20::StarknetErc20;
use kakarot_rpc_core::models::block::EthBlockId;
use kakarot_rpc_core::models::convertible::{
    ConvertibleEthEventFilter, ConvertibleStarknetEvent, ConvertibleStarknetTransaction,
    ConvertibleStarknetTransactionReceipt,
};
use kakarot_rpc_core::models::event::StarknetEvent;
use kakarot_rpc_core::models::event_filter::EthEventFilter;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_rpc_core::models::transaction::StarknetTransaction;
use kakarot_rpc_core::models::transaction_receipt::StarknetTransactionReceipt as TransactionReceiptWrapper;
use kakarot_rpc_core::models::ConversionError;
use reth_primitives::{
    AccessList, AccessListWithGasUsed, Address, BlockId, BlockNumberOrTag, Bytes, Signature, Transaction,
    TransactionSigned, TxEip1559, H256, H64, U128, U256, U64,
};
use reth_rlp::Decodable;
use reth_rpc_types::{
    CallRequest, EIP1186AccountProofResponse, FeeHistory, Filter, FilterChanges, Index, RichBlock, SyncInfo,
    SyncStatus, Transaction as EtherTransaction, TransactionKind, TransactionReceipt, TransactionRequest, Work,
};
use serde_json::Value;
use starknet::core::types::{
    BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction, Event, EventFilterWithPage, FieldElement,
    MaybePendingTransactionReceipt, ResultPageRequest, SyncStatusType,
    TransactionReceipt as StarknetTransactionReceipt,
};
use starknet::providers::Provider;

use crate::api::eth_api::EthApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct KakarotEthRpc<P: Provider + Send + Sync + 'static> {
    pub kakarot_client: Arc<KakarotClient<P>>,
}

impl<P: Provider + Send + Sync> KakarotEthRpc<P> {
    pub fn new(kakarot_client: Arc<KakarotClient<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> EthApiServer for KakarotEthRpc<P> {
    async fn block_number(&self) -> Result<U64> {
        let block_number =
            self.kakarot_client.starknet_provider().block_number().await.map_err(EthApiError::<P::Error>::from)?;
        Ok(block_number.into())
    }

    async fn syncing(&self) -> Result<SyncStatus> {
        let status = self.kakarot_client.starknet_provider().syncing().await.map_err(EthApiError::<P::Error>::from)?;

        match status {
            SyncStatusType::NotSyncing => Ok(SyncStatus::None),

            SyncStatusType::Syncing(data) => {
                let starting_block: U256 = U256::from(data.starting_block_num);
                let current_block: U256 = U256::from(data.current_block_num);
                let highest_block: U256 = U256::from(data.highest_block_num);
                let warp_chunks_amount: Option<U256> = None;
                let warp_chunks_processed: Option<U256> = None;

                let status_info = SyncInfo {
                    starting_block,
                    current_block,
                    highest_block,
                    warp_chunks_amount,
                    warp_chunks_processed,
                };

                Ok(SyncStatus::Info(status_info))
            }
        }
    }

    async fn coinbase(&self) -> Result<Address> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_coinbase".to_string()).into())
    }

    async fn accounts(&self) -> Result<Vec<Address>> {
        Ok(Vec::new())
    }

    async fn chain_id(&self) -> Result<Option<U64>> {
        Ok(Some(CHAIN_ID.into()))
    }

    async fn block_by_hash(&self, hash: H256, full: bool) -> Result<Option<RichBlock>> {
        let block_id = EthBlockId::new(BlockId::Hash(hash.into()));
        let starknet_block_id: StarknetBlockId = block_id.try_into().map_err(EthApiError::<P::Error>::from)?;
        let block = self.kakarot_client.get_eth_block_from_starknet_block(starknet_block_id, full).await?;
        Ok(Some(block))
    }

    async fn block_by_number(&self, number: BlockNumberOrTag, full: bool) -> Result<Option<RichBlock>> {
        let block_id = EthBlockId::new(BlockId::Number(number));
        let starknet_block_id: StarknetBlockId = block_id.try_into().map_err(EthApiError::<P::Error>::from)?;
        let block = self.kakarot_client.get_eth_block_from_starknet_block(starknet_block_id, full).await?;
        Ok(Some(block))
    }

    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64> {
        let block_id = BlockId::Hash(hash.into());
        self.kakarot_client.transaction_count_by_block(block_id).await.map_err(EthApiError::<P::Error>::into)
    }

    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<U64> {
        let block_id = BlockId::Number(number);
        self.kakarot_client.transaction_count_by_block(block_id).await.map_err(EthApiError::<P::Error>::into)
    }

    async fn block_uncles_count_by_block_hash(&self, _hash: H256) -> Result<U256> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getUncleCountByBlockHash".to_string()).into())
    }

    async fn block_uncles_count_by_block_number(&self, _number: BlockNumberOrTag) -> Result<U256> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getUncleCountByBlockNumber".to_string()).into())
    }

    async fn uncle_by_block_hash_and_index(&self, _hash: H256, _index: Index) -> Result<Option<RichBlock>> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getUncleByBlockHashAndIndex".to_string()).into())
    }

    async fn uncle_by_block_number_and_index(
        &self,
        _number: BlockNumberOrTag,
        _index: Index,
    ) -> Result<Option<RichBlock>> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getUncleByBlockNumberAndIndex".to_string()).into())
    }

    async fn transaction_by_hash(&self, hash: H256) -> Result<Option<EtherTransaction>> {
        let hash: Felt252Wrapper = hash.try_into().map_err(EthApiError::<P::Error>::from)?;
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
                let block_hash: Felt252Wrapper = tr.block_hash.into();
                (Some(block_hash.into()), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };
        let eth_transaction = transaction.to_eth_transaction(&self.kakarot_client, block_hash, block_num, None).await?;
        Ok(Some(eth_transaction))
    }

    async fn transaction_by_block_hash_and_index(&self, hash: H256, index: Index) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Hash(hash.into());
        let index = usize::from(index) as u64;
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;

        let starknet_tx: StarknetTransaction = self
            .kakarot_client
            .starknet_provider()
            .get_transaction_by_block_id_and_index(starknet_block_id, index)
            .await
            .map_err(EthApiError::<P::Error>::from)?
            .into();

        let tx_hash: FieldElement = starknet_tx.transaction_hash().map_err(EthApiError::<P::Error>::from)?.into();

        let tx_receipt = self
            .kakarot_client
            .starknet_provider()
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(EthApiError::<P::Error>::from)?;
        let (block_hash, block_num) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => {
                let block_hash: Felt252Wrapper = tr.block_hash.into();
                (Some(block_hash.into()), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };

        let eth_tx = starknet_tx
            .to_eth_transaction(&self.kakarot_client, block_hash, block_num, Some(U256::from(index)))
            .await?;
        Ok(Some(eth_tx))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number: BlockNumberOrTag,
        index: Index,
    ) -> Result<Option<EtherTransaction>> {
        let block_id = BlockId::Number(number);
        let index = usize::from(index) as u64;
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;

        let starknet_tx: StarknetTransaction = self
            .kakarot_client
            .starknet_provider()
            .get_transaction_by_block_id_and_index(starknet_block_id, index)
            .await
            .map_err(EthApiError::<P::Error>::from)?
            .into();

        let tx_hash: FieldElement = starknet_tx.transaction_hash().map_err(EthApiError::<P::Error>::from)?.into();

        let tx_receipt = self
            .kakarot_client
            .starknet_provider()
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(EthApiError::<P::Error>::from)?;
        let (block_hash, block_num) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => {
                let block_hash: Felt252Wrapper = tr.block_hash.into();
                (Some(block_hash.into()), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };

        let eth_tx = starknet_tx
            .to_eth_transaction(&self.kakarot_client, block_hash, block_num, Some(U256::from(index)))
            .await?;
        Ok(Some(eth_tx))
    }

    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>> {
        // TODO: Error when trying to transform 32 bytes hash to FieldElement
        let transaction_hash: Felt252Wrapper = hash.try_into().map_err(EthApiError::<P::Error>::from)?;
        let starknet_tx_receipt: TransactionReceiptWrapper = match self
            .kakarot_client
            .starknet_provider()
            .get_transaction_receipt::<FieldElement>(transaction_hash.into())
            .await
        {
            Err(_) => return Ok(None),
            Ok(receipt) => receipt,
        }
        .into();

        let receipt = starknet_tx_receipt.to_eth_transaction_receipt(&self.kakarot_client).await?;
        Ok(receipt)
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;
        let starknet_address = self.kakarot_client.compute_starknet_address(address, &starknet_block_id).await?;

        let native_token_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap();
        let provider = self.kakarot_client.starknet_provider();
        let native_token = StarknetErc20::new(&provider, native_token_address);
        let balance = native_token.balance_of(&starknet_address, &starknet_block_id).await?;

        Ok(balance)
    }

    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;

        let address: Felt252Wrapper = address.into();
        let address = address.into();

        let starknet_contract_address =
            self.kakarot_client.kakarot_contract().compute_starknet_address(&address, &starknet_block_id).await?;

        let key_low = index & U256::from(u128::MAX);
        let key_low: Felt252Wrapper = key_low.try_into().map_err(EthApiError::<P::Error>::from)?;

        let key_high = index >> 128;
        let key_high: Felt252Wrapper = key_high.try_into().map_err(EthApiError::<P::Error>::from)?;

        let contract_account = ContractAccount::new(starknet_contract_address, self.kakarot_client.starknet_provider());
        let storage_value = contract_account.storage(&key_low.into(), &key_high.into(), &starknet_block_id).await?;

        Ok(storage_value)
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));

        let transaction_count = self.kakarot_client.nonce(address, block_id).await?;

        Ok(transaction_count)
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> Result<Bytes> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;

        // Convert the hex-encoded string to a FieldElement
        let ethereum_address: Felt252Wrapper = address.into();
        let ethereum_address = ethereum_address.into();

        let starknet_contract_address = self
            .kakarot_client
            .kakarot_contract()
            .compute_starknet_address(&ethereum_address, &starknet_block_id)
            .await?;

        let contract_account = ContractAccount::new(starknet_contract_address, self.kakarot_client.starknet_provider());
        let bytecode = contract_account.bytecode(&starknet_block_id).await?;

        // Convert the result of the function call to a vector of bytes
        Ok(bytecode)
    }

    async fn get_logs(&self, filter: Filter) -> Result<FilterChanges> {
        // Check the block range
        let current_block: u64 =
            self.kakarot_client.starknet_provider().block_number().await.map_err(EthApiError::<P::Error>::from)?;
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
        let event_filter = filter.to_starknet_event_filter(&self.kakarot_client)?;

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
                let block_hash = {
                    let felt: Felt252Wrapper = emitted.block_hash.into();
                    felt.into()
                };
                let transaction_hash = {
                    let felt: Felt252Wrapper = emitted.transaction_hash.into();
                    felt.into()
                };
                event
                    .to_eth_log(
                        &self.kakarot_client,
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

    async fn call(&self, request: CallRequest, block_id: Option<BlockId>) -> Result<Bytes> {
        // unwrap option or return jsonrpc error
        let to = request.to.ok_or_else(|| {
            rpc_err(EthRpcErrorCode::InternalError, "CallRequest `to` field is None. Cannot process a Kakarot call")
        })?;

        // Here we check if CallRequest.origin is None, if so, we insert origin = address(0)
        let origin = request.from.unwrap_or(*TX_ORIGIN_ZERO);

        let calldata = request.input.data.ok_or_else(|| {
            rpc_err(EthRpcErrorCode::InternalError, "CallRequest `data` field is None. Cannot process a Kakarot call")
        })?;

        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;

        let to: Felt252Wrapper = to.into();
        let to = to.into();

        let origin: FieldElement = Felt252Wrapper::from(origin).into();

        let calldata = bytes_to_felt_vec(&calldata);

        let result =
            self.kakarot_client.kakarot_contract().eth_call(&origin, &to, calldata, &starknet_block_id).await?;

        Ok(result)
    }

    async fn create_access_list(
        &self,
        _request: CallRequest,
        _block_id: Option<BlockId>,
    ) -> Result<AccessListWithGasUsed> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_createAccessList".to_string()).into())
    }

    async fn estimate_gas(&self, request: CallRequest, block_id: Option<BlockId>) -> Result<U256> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));

        match self.kakarot_client.network() {
            Network::MainnetGateway | Network::Goerli1Gateway | Network::Goerli2Gateway => (),
            _ => {
                return Ok(*ESTIMATE_GAS);
            }
        };

        let chain_id = request.chain_id.unwrap_or(CHAIN_ID.into());

        let from = request
            .from
            .ok_or_else(|| EthApiError::<P::Error>::MissingParameterError("from for estimate_gas".into()))?;
        let nonce = self
            .kakarot_client
            .nonce(from, block_id)
            .await?
            .try_into()
            .map_err(ConversionError::<u64>::from)
            .map_err(EthApiError::<P::Error>::from)?;

        let gas_limit = request
            .gas
            .unwrap_or(U256::ZERO)
            .try_into()
            .map_err(ConversionError::<u64>::from)
            .map_err(EthApiError::<P::Error>::from)?;
        let max_fee_per_gas = request
            .max_fee_per_gas
            .unwrap_or_else(|| U256::from(BASE_FEE_PER_GAS))
            .try_into()
            .map_err(ConversionError::<u128>::from)
            .map_err(EthApiError::<P::Error>::from)?;
        let max_priority_fee_per_gas = request
            .max_priority_fee_per_gas
            .unwrap_or_else(|| U256::from(MAX_PRIORITY_FEE_PER_GAS))
            .try_into()
            .map_err(ConversionError::<u128>::from)
            .map_err(EthApiError::<P::Error>::from)?;

        let to = request.to.map_or(TransactionKind::Create, TransactionKind::Call);

        let value = request
            .value
            .unwrap_or(U256::ZERO)
            .try_into()
            .map_err(ConversionError::<u128>::from)
            .map_err(EthApiError::<P::Error>::from)?;

        let data = request.input.data.unwrap_or_default();

        let tx = Transaction::Eip1559(TxEip1559 {
            chain_id: chain_id.low_u64(),
            nonce,
            gas_limit,
            max_fee_per_gas,
            max_priority_fee_per_gas,
            to: to.into(),
            value,
            access_list: AccessList(vec![]),
            input: data,
        });

        let starknet_block_id: StarknetBlockId =
            EthBlockId::new(block_id).try_into().map_err(EthApiError::<P::Error>::from)?;
        let block_number = self.kakarot_client.map_block_id_to_block_number(&starknet_block_id).await?;

        let sender_address = self.kakarot_client.compute_starknet_address(from, &starknet_block_id).await?;

        let mut data = vec![];
        tx.encode_with_signature(&Signature::default(), &mut data, false);
        let data = data.into_iter().map(FieldElement::from).collect();
        let calldata = raw_kakarot_calldata(self.kakarot_client.kakarot_address(), data);

        let tx = BroadcastedInvokeTransaction {
            max_fee: FieldElement::ZERO,
            signature: vec![],
            sender_address,
            nonce: nonce.into(),
            calldata,
            is_query: false,
        };

        let fee_estimate = self.kakarot_client.simulate_transaction(tx, block_number, true).await?.fee_estimation;
        if fee_estimate.gas_usage < MINIMUM_GAS_FEE {
            return Ok(U256::from(MINIMUM_GAS_FEE));
        }
        Ok(U256::from(fee_estimate.gas_usage))
    }

    async fn gas_price(&self) -> Result<U256> {
        let gas_price = self.kakarot_client.base_fee_per_gas();
        Ok(gas_price)
    }

    async fn fee_history(
        &self,
        block_count: U256,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory> {
        let block_count_usize = usize::try_from(block_count)
            .map_err(|e| ConversionError::<()>::ValueOutOfRange(e.to_string()))
            .map_err(EthApiError::<P::Error>::from)?;

        let base_fee = self.kakarot_client.base_fee_per_gas();

        let base_fee_per_gas: Vec<U256> = vec![base_fee; block_count_usize + 1];
        let newest_block = match newest_block {
            BlockNumberOrTag::Number(n) => n,
            // TODO: Add Genesis block number
            BlockNumberOrTag::Earliest => 1_u64,
            _ => self.block_number().await?.as_u64(),
        };

        let gas_used_ratio: Vec<f64> = vec![0.9; block_count_usize];
        let newest_block = U256::from(newest_block);
        let oldest_block: U256 = if newest_block >= block_count { newest_block - block_count } else { U256::from(0) };

        // TODO: transition `reward` hardcoded default out of nearing-demo-day hack and seeing how to
        // properly source/translate this value
        Ok(FeeHistory { base_fee_per_gas, gas_used_ratio, oldest_block, reward: Some(vec![vec![]]) })
    }

    async fn max_priority_fee_per_gas(&self) -> Result<U128> {
        let max_priority_fee = MAX_PRIORITY_FEE_PER_GAS;
        Ok(max_priority_fee)
    }

    async fn mining(&self) -> Result<bool> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_mining".to_string()).into())
    }

    async fn hashrate(&self) -> Result<U256> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_hashrate".to_string()).into())
    }

    async fn get_work(&self) -> Result<Work> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getWork".to_string()).into())
    }

    async fn submit_hashrate(&self, _hashrate: U256, _id: H256) -> Result<bool> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_submitHashrate".to_string()).into())
    }

    async fn submit_work(&self, _nonce: H64, _pow_hash: H256, _mix_digest: H256) -> Result<bool> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_submitWork".to_string()).into())
    }

    async fn send_transaction(&self, _request: TransactionRequest) -> Result<H256> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_sendTransaction".to_string()).into())
    }

    async fn send_raw_transaction(&self, bytes: Bytes) -> Result<H256> {
        let mut data = bytes.as_ref();

        let transaction = TransactionSigned::decode(&mut data)
            .map_err(DataDecodingError::TransactionDecodingError)
            .map_err(EthApiError::<P::Error>::from)?;

        let evm_address = transaction.recover_signer().ok_or_else(|| {
            EthApiError::<P::Error>::Other(anyhow::anyhow!("Kakarot send_transaction: signature ecrecover failed"))
        })?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        let account_exists = self.kakarot_client.check_eoa_account_exists(evm_address, &starknet_block_id).await?;
        if !account_exists {
            let starknet_transaction_hash: FieldElement =
                Felt252Wrapper::from(self.kakarot_client.deploy_eoa(evm_address).await?).into();
            self.kakarot_client.wait_for_confirmation_on_l2(starknet_transaction_hash).await?;
        }

        let starknet_address = self.kakarot_client.compute_starknet_address(evm_address, &starknet_block_id).await?;

        let nonce = FieldElement::from(transaction.nonce());

        let calldata = raw_kakarot_calldata(self.kakarot_client.kakarot_address(), bytes_to_felt_vec(&bytes));

        // Get estimated_fee from Starknet
        // TODO right now this is set to 0 in order to avoid failure on max fee for
        // Katana.
        let max_fee = *MAX_FEE;

        let signature = vec![];

        let request = BroadcastedInvokeTransaction {
            max_fee,
            signature,
            nonce,
            sender_address: starknet_address,
            calldata,
            is_query: false,
        };

        let starknet_transaction_hash = self.kakarot_client.submit_starknet_transaction(request).await?;

        Ok(starknet_transaction_hash)
    }

    async fn sign(&self, _address: Address, _message: Bytes) -> Result<Bytes> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_sign".to_string()).into())
    }

    async fn sign_transaction(&self, _transaction: CallRequest) -> Result<Bytes> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_signTransaction".to_string()).into())
    }

    async fn sign_typed_data(&self, _address: Address, _data: Value) -> Result<Bytes> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_signTypedData".to_string()).into())
    }

    async fn get_proof(
        &self,
        _address: Address,
        _keys: Vec<H256>,
        _block_id: Option<BlockId>,
    ) -> Result<EIP1186AccountProofResponse> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getProof".to_string()).into())
    }

    async fn new_filter(&self, _filter: Filter) -> Result<U64> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_newFilter".to_string()).into())
    }

    async fn new_block_filter(&self) -> Result<U64> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_newBlockFilter".to_string()).into())
    }

    async fn new_pending_transaction_filter(&self) -> Result<U64> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_newPendingTransactionFilter".to_string()).into())
    }

    async fn uninstall_filter(&self, _id: U64) -> Result<bool> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_uninstallFilter".to_string()).into())
    }

    async fn get_filter_changes(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getFilterChanges".to_string()).into())
    }

    async fn get_filter_logs(&self, _id: U64) -> Result<FilterChanges> {
        Err(EthApiError::<P::Error>::MethodNotSupported("eth_getFilterLogs".to_string()).into())
    }
}
