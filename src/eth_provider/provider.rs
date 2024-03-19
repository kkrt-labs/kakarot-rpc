use alloy_rlp::Decodable as _;
use async_trait::async_trait;
use auto_impl::auto_impl;
use cainome::cairo_serde::CairoArrayLegacy;
use eyre::Result;
use itertools::Itertools;
use mongodb::bson::doc;
use reth_primitives::{constants::EMPTY_ROOT_HASH, revm_primitives::FixedBytes};
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, TransactionSigned, B256, U256, U64};
use reth_rpc_types::{
    other::OtherFields, Block, BlockHashOrNumber, BlockTransactions, FeeHistory, Filter, FilterChanges, Index,
    JsonStorageKey, RichBlock, TransactionReceipt, TransactionRequest, U64HexOrNumber, ValueOrArray,
};
use reth_rpc_types::{SyncInfo, SyncStatus};
use starknet::core::types::BroadcastedInvokeTransaction;
use starknet::core::types::SyncStatusType;
use starknet::core::utils::get_storage_var_address;
use starknet_crypto::FieldElement;

use super::constant::CALL_REQUEST_GAS_LIMIT;
use super::database::types::{
    header::StoredHeader, log::StoredLog, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
    transaction::StoredTransactionHash,
};
use super::database::Database;
use super::error::{EthApiError, EvmError, KakarotError, SignatureError, TransactionError};
use super::starknet::kakarot_core::{
    self,
    contract_account::ContractAccountReader,
    core::{KakarotCoreReader, Uint256},
    proxy::ProxyReader,
    starknet_address, to_starknet_transaction, CONTRACT_ACCOUNT_CLASS_HASH, EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH,
    KAKAROT_ADDRESS,
};
use super::starknet::{ERC20Reader, STARKNET_NATIVE_TOKEN};
use super::utils::{
    contract_not_found, entrypoint_not_found, into_filter, iter_into, split_u256, try_from_u8_iterator,
};
use crate::eth_provider::utils::format_hex;
use crate::models::block::EthBlockNumberOrTag;
use crate::models::felt::{ConversionError, Felt252Wrapper};
use crate::{into_via_try_wrapper, into_via_wrapper};

pub type EthProviderResult<T> = Result<T, EthApiError>;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait EthereumProvider {
    /// Returns the latest block number.
    async fn block_number(&self) -> EthProviderResult<U64>;
    /// Returns the syncing status.
    async fn syncing(&self) -> EthProviderResult<SyncStatus>;
    /// Returns the chain id.
    async fn chain_id(&self) -> EthProviderResult<Option<U64>>;
    /// Returns a block by hash. Block can be full or just the hashes of the transactions.
    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>>;
    /// Returns a block by number. Block can be full or just the hashes of the transactions.
    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>>;
    /// Returns the transaction count for a block by hash.
    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>>;
    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>>;
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<reth_rpc_types::Transaction>>;
    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>>;
    /// Returns the transaction by block number and index.
    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>>;
    /// Returns the transaction receipt by hash of the transaction.
    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>>;
    /// Returns the balance of an address in native eth.
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the storage of an address at a certain index.
    async fn storage_at(
        &self,
        address: Address,
        index: JsonStorageKey,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<B256>;
    /// Returns the nonce for the address at the given block.
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the code for the address at the given block.
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes>;
    /// Returns the logs for the given filter.
    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges>;
    /// Returns the result of a call.
    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<Bytes>;
    /// Returns the result of a estimate gas.
    async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the fee history given a block count and a newest block number.
    async fn fee_history(
        &self,
        block_count: U64HexOrNumber,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory>;
    /// Send a raw transaction to the network and returns the transactions hash.
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256>;
    async fn gas_price(&self) -> EthProviderResult<U256>;
    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<TransactionReceipt>>>;
}

/// Structure that implements the EthereumProvider trait.
/// Uses an access to a database to certain data, while
/// the rest is fetched from the Starknet Provider.
pub struct EthDataProvider<SP: starknet::providers::Provider> {
    database: Database,
    starknet_provider: SP,
}

#[async_trait]
impl<SP> EthereumProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn block_number(&self) -> EthProviderResult<U64> {
        let filter = doc! {};
        let sort = doc! { "header.number": -1 };
        let header: Option<StoredHeader> = self.database.get_one("headers", filter, sort).await?;
        let block_number = match header {
            None => U64::from(self.starknet_provider.block_number().await.map_err(KakarotError::from)?), // in case the database is empty, use the starknet provider
            Some(header) => {
                let number = header.header.number.ok_or(EthApiError::UnknownBlockNumber)?;
                let number: u64 = number
                    .try_into()
                    .inspect_err(|err| tracing::error!("internal error: {:?}", err))
                    .map_err(|_| EthApiError::UnknownBlockNumber)?;
                U64::from(number)
            }
        };
        Ok(block_number)
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        let syncing_status = self.starknet_provider.syncing().await.map_err(KakarotError::from)?;

        match syncing_status {
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

    // TODO cache chain id
    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        let chain_id = self.starknet_provider.chain_id().await.map_err(KakarotError::from)?;
        let chain_id: Option<u64> = chain_id.try_into().ok();
        Ok(chain_id.map(U64::from))
    }

    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        let block = self.block(BlockHashOrNumber::Hash(hash), full).await?;

        Ok(block)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let block = self.block(BlockHashOrNumber::Number(block_number.to::<u64>()), full).await?;

        Ok(block)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        let block_exists = self.block_exists(BlockHashOrNumber::Hash(hash)).await?;
        if !block_exists {
            return Ok(None);
        }

        let filter = into_filter("tx.blockHash", hash, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(Some(U256::from(count)))
    }

    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let block_exists = self.block_exists(BlockHashOrNumber::Number(block_number.to::<u64>())).await?;
        if !block_exists {
            return Ok(None);
        }

        let filter = into_filter("tx.blockNumber", block_number, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(Some(U256::from(count)))
    }

    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let filter = into_filter("tx.hash", hash, 64);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let mut filter = into_filter("tx.blockHash", hash, 64);
        let index: usize = index.into();

        filter.insert("tx.transactionIndex", format_hex(index, 64));
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let mut filter = into_filter("tx.blockNumber", block_number, 64);
        let index: usize = index.into();

        filter.insert("tx.transactionIndex", format_hex(index, 64));
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        let filter = into_filter("receipt.transactionHash", hash, 64);
        let tx: Option<StoredTransactionReceipt> = self.database.get_one("receipts", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, &self.starknet_provider);

        let address = starknet_address(address);
        let balance = eth_contract
            .balanceOf(&address)
            .block_id(starknet_block_id)
            .call()
            .await
            .map_err(KakarotError::from)?
            .balance;

        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);
        Ok(low + (high << 128))
    }

    async fn storage_at(
        &self,
        address: Address,
        index: JsonStorageKey,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<B256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);

        let keys = split_u256::<FieldElement>(index.0);
        let storage_address = get_storage_var_address("storage_", &keys).expect("Storage var name is not ASCII");

        let storage = contract
            .storage(&storage_address)
            .block_id(starknet_block_id)
            .call()
            .await
            .map_err(KakarotError::from)?
            .value;

        let low: U256 = into_via_wrapper!(storage.low);
        let high: U256 = into_via_wrapper!(storage.high);
        let storage: U256 = low + (high << 128);

        Ok(storage.into())
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let proxy = ProxyReader::new(address, &self.starknet_provider);
        let maybe_class_hash = proxy.get_implementation().block_id(starknet_block_id).call().await;

        if contract_not_found(&maybe_class_hash) {
            return Ok(U256::ZERO);
        }
        let class_hash = maybe_class_hash.map_err(KakarotError::from)?.implementation;

        let nonce = if class_hash == *EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH {
            self.starknet_provider.get_nonce(starknet_block_id, address).await.map_err(KakarotError::from)?
        } else if class_hash == *CONTRACT_ACCOUNT_CLASS_HASH {
            let contract = ContractAccountReader::new(address, &self.starknet_provider);
            contract.get_nonce().block_id(starknet_block_id).call().await.map_err(KakarotError::from)?.nonce
        } else {
            FieldElement::ZERO
        };
        Ok(into_via_wrapper!(nonce))
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);
        let bytecode = contract.bytecode().block_id(starknet_block_id).call().await;

        if contract_not_found(&bytecode) || entrypoint_not_found(&bytecode) {
            return Ok(Bytes::default());
        }

        let bytecode = bytecode.map_err(KakarotError::from)?.bytecode.0;
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<u8>>(bytecode)))
    }

    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        let current_block = self.block_number().await?.try_into().map_err(|_| EthApiError::UnknownBlockNumber)?;
        let from = filter.get_from_block().unwrap_or_default();
        let to = filter.get_to_block().unwrap_or(current_block);

        let (from, to) = match (from, to) {
            (from, _) if from > current_block => return Ok(FilterChanges::Empty),
            (from, to) if to > current_block => (from, current_block),
            (from, to) if to < from => return Ok(FilterChanges::Empty),
            _ => (from, to),
        };

        // Convert the topics to a vector of B256
        let topics = filter
            .topics
            .into_iter()
            .filter_map(|t| t.to_value_or_array())
            .flat_map(|t| match t {
                ValueOrArray::Value(topic) => vec![topic],
                ValueOrArray::Array(topics) => topics,
            })
            .collect::<Vec<_>>();

        // Create the database filter. We filter by block number using $gte and $lte,
        // and by topics using $expr and $eq. The topics query will:
        // 1. Slice the topics array to the same length as the filter topics
        // 2. Match on values for which the sliced topics equal the filter topics
        let mut database_filter = doc! {
            "log.blockNumber": {"$gte": format_hex(from, 64), "$lte": format_hex(to, 64)},
            "$expr": {
                "$eq": [
                  { "$slice": ["$log.topics", topics.len() as i32] },
                  topics.into_iter().map(|t| format_hex(t, 64)).collect::<Vec<_>>()
                ]
              }
        };

        // Add the address filter if any
        let addresses = filter.address.to_value_or_array().map(|a| match a {
            ValueOrArray::Value(address) => vec![address],
            ValueOrArray::Array(addresses) => addresses,
        });
        addresses.map(|adds| {
            database_filter
                .insert("log.address", doc! {"$in": adds.into_iter().map(|a| format_hex(a, 40)).collect::<Vec<_>>()})
        });

        let logs: Vec<StoredLog> = self.database.get("logs", database_filter, None).await?;
        Ok(FilterChanges::Logs(logs.into_iter().map_into().collect()))
    }

    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let (output, _) = self.call_helper(request, block_id).await?;
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<_>>(output.0)))
    }

    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        // Set a high gas limit to make sure the transaction will not fail due to gas.
        let request = TransactionRequest { gas: Some(U256::from(u64::MAX)), ..request };

        let (_, gas_used) = self.call_helper(request, block_id).await?;
        Ok(U256::from(gas_used))
    }

    async fn fee_history(
        &self,
        block_count: U64HexOrNumber,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory> {
        if block_count.to() == 0 {
            return Ok(FeeHistory::default());
        }

        let end_block = self.tag_into_block_number(newest_block).await?;
        let end_block = end_block.to::<u64>();
        let end_block_plus = end_block.saturating_add(1);

        // 0 <= start_block <= end_block
        let start_block = end_block_plus.saturating_sub(block_count.to());

        // TODO: check if we should use a projection since we only need the gasLimit and gasUsed.
        // This means we need to introduce a new type for the StoredHeader.
        let header_filter =
            doc! {"header.number": {"$gte": format_hex(start_block, 64), "$lte": format_hex(end_block, 64)}};
        let blocks: Vec<StoredHeader> = self.database.get("headers", header_filter, None).await?;

        if blocks.is_empty() {
            return Err(EthApiError::UnknownBlock);
        }

        let gas_used_ratio = blocks
            .iter()
            .map(|header| {
                let gas_used = header.header.gas_used.as_limbs()[0] as f64;
                let gas_limit = if header.header.gas_limit != U256::ZERO {
                    header.header.gas_limit.as_limbs()[0] as f64
                } else {
                    1.0
                };
                gas_used / gas_limit
            })
            .collect::<Vec<_>>();

        let mut base_fee_per_gas =
            blocks.iter().map(|header| header.header.base_fee_per_gas.unwrap_or_default()).collect::<Vec<_>>();
        // TODO(EIP1559): Remove this when proper base fee computation: if gas_ratio > 50%, increase base_fee_per_gas
        base_fee_per_gas.extend_from_within((base_fee_per_gas.len() - 1)..);

        Ok(FeeHistory {
            base_fee_per_gas,
            gas_used_ratio,
            oldest_block: U256::from(start_block),
            reward: Some(vec![]),
            ..Default::default()
        })
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256> {
        let mut data = transaction.0.as_ref();
        let transaction_signed =
            TransactionSigned::decode(&mut data).map_err(|_| EthApiError::TransactionConversionError)?;

        let chain_id =
            self.chain_id().await?.unwrap_or_default().try_into().map_err(|_| TransactionError::InvalidChainId)?;

        let signer = transaction_signed.recover_signer().ok_or(SignatureError::RecoveryError)?;

        let max_fee: u64;
        #[cfg(not(feature = "hive"))]
        {
            // TODO(Kakarot Fee Mechanism): When we no longer need to use the Starknet fees, remove this line.
            // We need to get the balance (in Kakarot/Starknet native Token) of the signer to compute the Starknet maximum `max_fee`.
            // We used to set max_fee = u64::MAX, but it'll fail if the signer doesn't have enough balance to pay the fees.
            let eth_fees_per_gas =
                transaction_signed.effective_gas_price(Some(transaction_signed.max_fee_per_gas() as u64)) as u64;
            let eth_fees = eth_fees_per_gas.saturating_mul(transaction_signed.gas_limit());
            let balance = self.balance(signer, None).await?;
            max_fee = {
                let max_fee: u64 = balance.try_into().unwrap_or(u64::MAX);
                max_fee.saturating_sub(eth_fees)
            };
        }
        #[cfg(feature = "hive")]
        {
            max_fee = u64::MAX;
        }

        let transaction = to_starknet_transaction(&transaction_signed, chain_id, signer, max_fee)?;

        // If the contract is not found, we need to deploy it.
        #[cfg(feature = "hive")]
        {
            use crate::eth_provider::constant::{DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
            use starknet::accounts::Call;
            use starknet::accounts::Execution;
            use starknet::core::types::BlockTag;
            use starknet::core::utils::get_selector_from_name;
            let sender = transaction.sender_address;
            let proxy = ProxyReader::new(sender, &self.starknet_provider);
            let maybe_class_hash =
                proxy.get_implementation().block_id(starknet::core::types::BlockId::Tag(BlockTag::Latest)).call().await;

            if contract_not_found(&maybe_class_hash) {
                let execution = Execution::new(
                    vec![Call {
                        to: *KAKAROT_ADDRESS,
                        selector: get_selector_from_name("deploy_externally_owned_account").unwrap(),
                        calldata: vec![into_via_wrapper!(signer)],
                    }],
                    &*DEPLOY_WALLET,
                );

                let mut nonce = DEPLOY_WALLET_NONCE.lock().await;
                let current_nonce = *nonce;

                let tx = execution
                    .nonce(current_nonce)
                    .max_fee(FieldElement::from(u64::MAX))
                    .prepared()
                    .map_err(|_| EthApiError::TransactionConversionError)?
                    .get_invoke_request(false)
                    .await
                    .map_err(|_| SignatureError::SignError)?;
                self.starknet_provider.add_invoke_transaction(tx).await.map_err(KakarotError::from)?;

                *nonce += 1u8.into();
                drop(nonce);
            };
        }

        #[cfg(not(feature = "testing"))]
        {
            let hash = transaction_signed.hash();
            let tx = self
                .starknet_provider
                .add_invoke_transaction(BroadcastedInvokeTransaction::V1(transaction))
                .await
                .map_err(KakarotError::from)?;
            tracing::info!(
                "Fired a transaction: Starknet Hash: {:?} --- Ethereum Hash: {:?}",
                tx.transaction_hash,
                hash
            );
            Ok(hash)
        }
        // If we are currently testing, we need to return the starknet hash in order
        // to be able to wait for the transaction to be mined.
        #[cfg(feature = "testing")]
        {
            let res = self
                .starknet_provider
                .add_invoke_transaction(BroadcastedInvokeTransaction::V1(transaction))
                .await
                .map_err(KakarotError::from)?;
            Ok(B256::from_slice(&res.transaction_hash.to_bytes_be()[..]))
        }
    }

    async fn gas_price(&self) -> EthProviderResult<U256> {
        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let gas_price = kakarot_contract.get_base_fee().call().await.map_err(KakarotError::from)?.base_fee;
        Ok(into_via_wrapper!(gas_price))
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<TransactionReceipt>>> {
        let block_id = block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest));
        match block_id {
            BlockId::Number(maybe_number) => {
                let block_number = self.tag_into_block_number(maybe_number).await?;
                let block_exists = self.block_exists(BlockHashOrNumber::Number(block_number.to())).await?;
                if !block_exists {
                    return Ok(None);
                }

                let filter = into_filter("receipt.blockNumber", block_number, 64);
                let tx: Vec<StoredTransactionReceipt> = self.database.get("receipts", filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
            BlockId::Hash(hash) => {
                let block_exists = self.block_exists(BlockHashOrNumber::Hash(hash.block_hash)).await?;
                if !block_exists {
                    return Ok(None);
                }

                let filter = into_filter("receipt.blockHash", hash.block_hash, 64);
                let tx: Vec<StoredTransactionReceipt> = self.database.get("receipts", filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
        }
    }
}

impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    pub const fn new(database: Database, starknet_provider: SP) -> Self {
        Self { database, starknet_provider }
    }

    #[cfg(feature = "testing")]
    pub fn starknet_provider(&self) -> &SP {
        &self.starknet_provider
    }

    async fn call_helper(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<(CairoArrayLegacy<FieldElement>, u128)> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        // unwrap option
        let to: kakarot_core::core::Option = {
            match request.to {
                Some(to) => kakarot_core::core::Option { is_some: FieldElement::ONE, value: into_via_wrapper!(to) },
                None => kakarot_core::core::Option { is_some: FieldElement::ZERO, value: FieldElement::ZERO },
            }
        };

        // Here we check if CallRequest.origin is None, if so, we insert origin = address(0)
        let from = into_via_wrapper!(request.from.unwrap_or_default());

        let data = request.input.into_input().unwrap_or_default();
        let calldata: Vec<FieldElement> = data.into_iter().map_into().collect();

        let gas_limit = into_via_try_wrapper!(request.gas.unwrap_or_else(|| U256::from(CALL_REQUEST_GAS_LIMIT)))
            .map_err(KakarotError::from)?;

        // We cannot unwrap_or_default() here because Kakarot.eth_call will
        // Reject transactions with gas_price < Kakarot.base_fee
        let gas_price = {
            let gas_price = match request.gas_price {
                Some(gas_price) => gas_price,
                None => self.gas_price().await?,
            };
            into_via_try_wrapper!(gas_price).map_err(KakarotError::from)?
        };

        let value = into_via_try_wrapper!(request.value.unwrap_or_default()).map_err(KakarotError::from)?;

        // TODO: replace this by into_via_wrapper!(request.nonce.unwrap_or_default())
        //  when we can simulate the transaction instead of calling `eth_call`
        let nonce = {
            match request.nonce {
                Some(nonce) => into_via_wrapper!(nonce),
                None => match request.from {
                    None => FieldElement::ZERO,
                    Some(address) => into_via_try_wrapper!(self.transaction_count(address, block_id).await?)
                        .map_err(KakarotError::from)?,
                },
            }
        };

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let call_output = kakarot_contract
            .eth_call(
                &nonce,
                &from,
                &to,
                &gas_limit,
                &gas_price,
                &Uint256 { low: value, high: FieldElement::ZERO },
                &calldata.len().into(),
                &CairoArrayLegacy(calldata),
                &FieldElement::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .await
            .map_err(KakarotError::from)?;

        let return_data = call_output.return_data;
        if call_output.success == FieldElement::ZERO {
            let revert_reason =
                return_data.0.into_iter().filter_map(|x| u8::try_from(x).ok()).map(|x| x as char).collect::<String>();
            return Err(KakarotError::from(EvmError::from(revert_reason)).into());
        }
        let gas_used = call_output.gas_used.try_into().map_err(|_| TransactionError::GasOverflow)?;
        Ok((return_data, gas_used))
    }

    /// Check if a block exists in the database.
    async fn block_exists(&self, block_id: BlockHashOrNumber) -> EthProviderResult<bool> {
        Ok(self.header(block_id).await?.is_some())
    }

    /// Get a header from the database based on the filter.
    async fn header(&self, id: BlockHashOrNumber) -> EthProviderResult<Option<StoredHeader>> {
        let filter = match id {
            BlockHashOrNumber::Hash(hash) => into_filter("header.hash", hash, 64),
            BlockHashOrNumber::Number(number) => into_filter("header.number", number, 64),
        };
        self.database
            .get_one("headers", filter, None)
            .await
            .inspect_err(|err| {
                tracing::error!("internal error: {:?}", err);
            })
            .map_err(|_| EthApiError::UnknownBlock)
    }

    /// Get a block from the database based on a block hash or number.
    /// If full is true, the block will contain the full transactions, otherwise just the hashes
    async fn block(&self, block_id: BlockHashOrNumber, full: bool) -> EthProviderResult<Option<RichBlock>> {
        let header = self.header(block_id).await?;
        let header = match header {
            Some(header) => header,
            None => return Ok(None),
        };
        let total_difficulty = Some(header.header.difficulty);

        let transactions_filter = match block_id {
            BlockHashOrNumber::Hash(hash) => into_filter("tx.blockHash", hash, 64),
            BlockHashOrNumber::Number(number) => into_filter("tx.blockNumber", number, 64),
        };

        let transactions = if full {
            BlockTransactions::Full(iter_into(
                self.database.get::<StoredTransaction>("transactions", transactions_filter, None).await?,
            ))
        } else {
            BlockTransactions::Hashes(iter_into(
                self.database
                    .get::<StoredTransactionHash>("transactions", transactions_filter, doc! {"tx.hash": 1})
                    .await?,
            ))
        };

        // The withdrawals are not supported, hence the withdrawals_root should always be empty.
        let withdrawal_root = header.header.withdrawals_root.unwrap_or_default();
        if withdrawal_root != EMPTY_ROOT_HASH {
            return Err(EthApiError::Unsupported("withdrawals"));
        }

        let block = Block {
            header: header.header,
            transactions,
            total_difficulty,
            uncles: Vec::new(),
            size: None,
            withdrawals: Some(vec![]),
            other: OtherFields::default(),
        };

        Ok(Some(block.into()))
    }

    /// Convert the given block id into a Starknet block id
    pub async fn to_starknet_block_id(
        &self,
        block_id: impl Into<Option<BlockId>>,
    ) -> EthProviderResult<starknet::core::types::BlockId> {
        match block_id.into() {
            Some(BlockId::Hash(hash)) => {
                let header = self.header(BlockHashOrNumber::Hash(hash.block_hash)).await?;
                let n = header.ok_or(EthApiError::UnknownBlock)?.header.number.ok_or(EthApiError::UnknownBlock)?;
                Ok(starknet::core::types::BlockId::Number(
                    n.try_into().map_err(|_| KakarotError::from(ConversionError))?,
                ))
            }
            Some(BlockId::Number(number_or_tag)) => {
                // There is a need to separate the BlockNumberOrTag case into three subcases
                // because pending Starknet blocks don't have a number.
                // 1. The block number corresponds to a Starknet pending block, then we return the pending tag
                // 2. The block number corresponds to a Starknet sealed block, then we return the block number
                // 3. The block number is not found, then we return an error
                match number_or_tag {
                    BlockNumberOrTag::Number(number) => {
                        let header = self
                            .header(BlockHashOrNumber::Number(number))
                            .await?
                            .ok_or(EthApiError::UnknownBlockNumber)?;
                        // If the block hash is zero, then the block corresponds to a Starknet pending block
                        if header.header.hash.ok_or(EthApiError::UnknownBlock)? == FixedBytes::ZERO {
                            Ok(starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending))
                        } else {
                            Ok(starknet::core::types::BlockId::Number(number))
                        }
                    }
                    _ => Ok(EthBlockNumberOrTag::from(number_or_tag).into()),
                }
            }
            None => Ok(starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending)),
        }
    }

    /// Convert the given BlockNumberOrTag into a block number
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        match tag {
            BlockNumberOrTag::Earliest => Ok(U64::ZERO),
            BlockNumberOrTag::Number(number) => Ok(U64::from(number)),
            BlockNumberOrTag::Latest
            | BlockNumberOrTag::Finalized
            | BlockNumberOrTag::Safe
            | BlockNumberOrTag::Pending => self.block_number().await,
        }
    }
}
