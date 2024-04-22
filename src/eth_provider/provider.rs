use crate::eth_provider::starknet::kakarot_core::account_contract::BytecodeOutput;
use crate::models::block::rpc_to_primitive_header;
use alloy_rlp::{Decodable, Encodable};
use async_trait::async_trait;
use auto_impl::auto_impl;
use cainome::cairo_serde::CairoArrayLegacy;
use cainome::cairo_serde::CairoSerde;
use eyre::Result;
use itertools::Itertools;
use mongodb::bson::doc;
use reth_primitives::constants::EMPTY_ROOT_HASH;
use reth_primitives::serde_helper::{JsonStorageKey, U64HexOrNumber};
use reth_primitives::{
    Address, BlockId, BlockNumberOrTag, Bytes, TransactionSigned, TransactionSignedEcRecovered, B256, U256, U64,
};
use reth_rpc_types::{
    Block, BlockHashOrNumber, BlockTransactions, FeeHistory, Filter, FilterChanges, Header, Index, RichBlock,
    TransactionReceipt, TransactionRequest, ValueOrArray,
};
use reth_rpc_types::{SyncInfo, SyncStatus};
use reth_rpc_types_compat::transaction::from_recovered;
use starknet::core::types::{FunctionCall, SyncStatusType};
use starknet::core::utils::{get_selector_from_name, get_storage_var_address};
use starknet_crypto::FieldElement;

use super::constant::{CALL_REQUEST_GAS_LIMIT, HASH_PADDING, U64_PADDING};
use super::database::types::{
    header::StoredHeader, log::StoredLog, receipt::StoredTransactionReceipt, transaction::StoredPendingTransaction,
    transaction::StoredTransaction, transaction::StoredTransactionHash,
};
use super::database::{CollectionName, Database};
use super::error::{EthApiError, EthereumDataFormatError, EvmError, KakarotError, SignatureError, TransactionError};
use super::starknet::kakarot_core::{
    self,
    account_contract::AccountContractReader,
    core::KakarotCoreReader,
    core::{CallInput, Uint256},
    starknet_address, to_starknet_transaction, KAKAROT_ADDRESS,
};
use super::starknet::{ERC20Reader, STARKNET_NATIVE_TOKEN};
use super::utils::{contract_not_found, entrypoint_not_found, into_filter, split_u256, try_from_u8_iterator};
use crate::eth_provider::utils::format_hex;
use crate::models::block::{EthBlockId, EthBlockNumberOrTag};
use crate::models::felt::Felt252Wrapper;
use crate::{into_via_try_wrapper, into_via_wrapper};

pub type EthProviderResult<T> = Result<T, EthApiError>;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc, &)]
pub trait EthereumProvider {
    /// Get header by block id
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>>;
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
    /// Returns the current gas price.
    async fn gas_price(&self) -> EthProviderResult<U256>;
    /// Returns the block receipts for a block.
    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<TransactionReceipt>>>;
    /// Returns the transactions for a block.
    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>>;
}

/// Structure that implements the EthereumProvider trait.
/// Uses an access to a database to certain data, while
/// the rest is fetched from the Starknet Provider.
#[derive(Debug)]
pub struct EthDataProvider<SP: starknet::providers::Provider> {
    database: Database,
    starknet_provider: SP,
    chain_id: u64,
}

impl<SP: starknet::providers::Provider> EthDataProvider<SP> {
    /// Returns a reference to the database.
    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl<SP> EthereumProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>> {
        let block = match block_id {
            BlockId::Hash(hash) => BlockHashOrNumber::Hash((*hash).into()),
            BlockId::Number(number_or_tag) => self.tag_into_block_number(*number_or_tag).await?.to::<u64>().into(),
        };

        Ok(self.header(block).await?.map(|h| h.header))
    }

    async fn block_number(&self) -> EthProviderResult<U64> {
        let sort = doc! { "header.number": -1 };
        let block_number = match self.database.get_one::<StoredHeader>(None, sort).await? {
            // In case the database is empty, use the starknet provider
            None => U64::from(self.starknet_provider.block_number().await.map_err(KakarotError::from)?),
            Some(header) => {
                let number = header.header.number.ok_or(EthApiError::UnknownBlockNumber)?;
                let number: u64 = number
                    .try_into()
                    .inspect_err(|err| tracing::error!("internal error: {:?}", err))
                    .map_err(|_| EthApiError::UnknownBlockNumber)?;

                let is_pending_block = header.header.hash.unwrap_or_default().is_zero();
                U64::from(if is_pending_block { number - 1 } else { number })
            }
        };
        Ok(block_number)
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        Ok(match self.starknet_provider.syncing().await.map_err(KakarotError::from)? {
            SyncStatusType::NotSyncing => SyncStatus::None,
            SyncStatusType::Syncing(data) => SyncStatus::Info(SyncInfo {
                starting_block: U256::from(data.starting_block_num),
                current_block: U256::from(data.current_block_num),
                highest_block: U256::from(data.highest_block_num),
                ..Default::default()
            }),
        })
    }

    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        Ok(Some(U64::from(self.chain_id)))
    }

    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        Ok(self.block(hash.into(), full).await?)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        Ok(self.block(block_number.to::<u64>().into(), full).await?)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        Ok(if self.block_exists(hash.into()).await? {
            Some(U256::from(
                self.database.count::<StoredTransaction>(into_filter("tx.blockHash", &hash, HASH_PADDING)).await?,
            ))
        } else {
            None
        })
    }

    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let block_exists = self.block_exists(block_number.into()).await?;
        if !block_exists {
            return Ok(None);
        }

        let filter = into_filter("tx.blockNumber", &block_number, U64_PADDING);
        let count = self.database.count::<StoredTransaction>(filter).await?;
        Ok(Some(U256::from(count)))
    }

    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let pipeline = vec![
            doc! {
                // Union with pending transactions with only specified hash
                "$unionWith": {
                    "coll": StoredPendingTransaction::collection_name(),
                    "pipeline": [
                        {
                            "$match": {
                                "tx.hash": format_hex(hash, HASH_PADDING)
                            }
                        }
                    ]
                },
            },
            // Only specified hash in the transactions collection
            doc! {
                "$match": {
                    "tx.hash": format_hex(hash, HASH_PADDING)
                }
            },
            // Sort in descending order by block number as pending transactions have null block number
            doc! {
                "$sort": { "tx.blockNumber" : -1 }
            },
            // Only one document in the final result with priority to the final transactions collection if available
            doc! {
                "$limit": 1
            },
        ];

        Ok(self.database.get_one_aggregate::<StoredTransaction>(pipeline).await?.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let mut filter = into_filter("tx.blockHash", &hash, HASH_PADDING);
        let index: usize = index.into();

        filter.insert("tx.transactionIndex", format_hex(index, U64_PADDING));
        Ok(self.database.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let mut filter = into_filter("tx.blockNumber", &block_number, U64_PADDING);
        let index: usize = index.into();

        filter.insert("tx.transactionIndex", format_hex(index, U64_PADDING));
        Ok(self.database.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        Ok(self
            .database
            .get_one::<StoredTransactionReceipt>(into_filter("receipt.transactionHash", &hash, HASH_PADDING), None)
            .await?
            .map(Into::into))
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
        let contract = AccountContractReader::new(address, &self.starknet_provider);

        let keys = split_u256::<FieldElement>(index.0);
        let storage_address = get_storage_var_address("Account_storage", &keys).expect("Storage var name is not ASCII");

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
        let account_contract = AccountContractReader::new(address, &self.starknet_provider);
        let maybe_nonce = account_contract.get_nonce().block_id(starknet_block_id).call().await;

        if contract_not_found(&maybe_nonce) {
            return Ok(U256::ZERO);
        }
        let nonce = maybe_nonce.map_err(KakarotError::from)?.nonce;

        // Get the protocol nonce as well, in edge cases where the protocol nonce is higher than the account nonce.
        // This can happen when an underlying Starknet transaction reverts => Account storage changes are reverted,
        // but the protocol nonce is still incremented.
        let protocol_nonce =
            self.starknet_provider.get_nonce(starknet_block_id, address).await.map_err(KakarotError::from)?;
        let nonce = nonce.max(protocol_nonce);

        Ok(into_via_wrapper!(nonce))
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        // TODO: temporary fix to handle empty bytecode until
        // https://github.com/cartridge-gg/cainome/issues/24 is solved
        let bytecode = self
            .starknet_provider
            .call(
                FunctionCall {
                    contract_address: starknet_address(address),
                    entry_point_selector: get_selector_from_name("bytecode").unwrap(),
                    calldata: Default::default(),
                },
                self.to_starknet_block_id(block_id).await?,
            )
            .await
            .map_err(cainome::cairo_serde::Error::Provider);

        if contract_not_found(&bytecode) || entrypoint_not_found(&bytecode) {
            return Ok(Bytes::default());
        }

        let bytecode_raw = bytecode.map_err(KakarotError::from)?;

        // If the bytecode is empty, return an empty Bytes
        if bytecode_raw.len() == 1 && bytecode_raw[0] == FieldElement::ZERO {
            return Ok(Bytes::default());
        }

        // Deserialize the bytecode
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<u8>>(
            BytecodeOutput::cairo_deserialize(&bytecode_raw, 0).map_err(KakarotError::from)?.bytecode.0,
        )))
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

        let logs = self.database.get::<StoredLog>(database_filter, None).await?;
        Ok(FilterChanges::Logs(logs.into_iter().map_into().collect()))
    }

    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let output = self.call_helper(request, block_id).await?;
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<_>>(output.0)))
    }

    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        // Set a high gas limit to make sure the transaction will not fail due to gas.
        let request = TransactionRequest { gas: Some(U256::from(u64::MAX)), ..request };

        let gas_used = self.estimate_gas_helper(request, block_id).await?;
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
        let header_filter = doc! {"$and": [ { "header.number": { "$gte": format_hex(start_block, U64_PADDING) } }, { "header.number": { "$lte": format_hex(end_block, U64_PADDING) } } ] };
        let blocks: Vec<StoredHeader> = self.database.get(header_filter, None).await?;

        if blocks.is_empty() {
            return Err(EthApiError::UnknownBlock);
        }

        let gas_used_ratio = blocks
            .iter()
            .map(|header| {
                let gas_used = header.header.gas_used.as_limbs()[0] as f64;
                let mut gas_limit = header.header.gas_limit.as_limbs()[0] as f64;
                if gas_limit == 0. {
                    gas_limit = 1.;
                };
                gas_used / gas_limit
            })
            .collect();

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
        // Get the chain ID
        let chain_id =
            self.chain_id().await?.unwrap_or_default().try_into().map_err(|_| TransactionError::InvalidChainId)?;

        // Decode the transaction data
        let transaction_signed = TransactionSigned::decode(&mut transaction.0.as_ref())
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversionError))?;

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::RecoveryError)?;

        // Determine the maximum fee
        let max_fee = if cfg!(feature = "hive") {
            u64::MAX
        } else {
            // TODO(Kakarot Fee Mechanism): When we no longer need to use the Starknet fees, remove this line.
            // We need to get the balance (in Kakarot/Starknet native Token) of the signer to compute the Starknet maximum `max_fee`.
            // We used to set max_fee = u64::MAX, but it'll fail if the signer doesn't have enough balance to pay the fees.
            let eth_fees_per_gas =
                transaction_signed.effective_gas_price(Some(transaction_signed.max_fee_per_gas() as u64)) as u64;
            let eth_fees = eth_fees_per_gas.saturating_mul(transaction_signed.gas_limit());
            let balance = self.balance(signer, None).await?;
            let max_fee: u64 = balance.try_into().unwrap_or(u64::MAX);
            max_fee.saturating_sub(eth_fees)
        };

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.deploy_evm_transaction_signer(signer).await?;

        // Convert the transaction to a Starknet transaction
        let transaction = to_starknet_transaction(&transaction_signed, chain_id, signer, max_fee)?;

        // Add the transaction to the Starknet provider
        let res = self.starknet_provider.add_invoke_transaction(transaction).await.map_err(KakarotError::from)?;

        // Serialize transaction document
        let transaction =
            from_recovered(TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer));

        // Update pending transactions collection
        let filter = into_filter("tx.hash", &transaction.hash, HASH_PADDING);
        self.database.update_one::<StoredPendingTransaction>(transaction.into(), filter, true).await?;

        // Return transaction hash if testing feature is enabled, otherwise log and return Ethereum hash
        if cfg!(feature = "testing") {
            return Ok(B256::from_slice(&res.transaction_hash.to_bytes_be()[..]));
        } else {
            let hash = transaction_signed.hash();
            tracing::info!(
                "Fired a transaction: Starknet Hash: {:?} --- Ethereum Hash: {:?}",
                res.transaction_hash,
                hash
            );

            Ok(hash)
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
                let block_exists = self.block_exists(block_number.into()).await?;
                if !block_exists {
                    return Ok(None);
                }

                let filter = into_filter("receipt.blockNumber", &block_number, U64_PADDING);
                let tx: Vec<StoredTransactionReceipt> = self.database.get(filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
            BlockId::Hash(hash) => {
                let block_exists = self.block_exists(hash.block_hash.into()).await?;
                if !block_exists {
                    return Ok(None);
                }

                let filter = into_filter("receipt.blockHash", &hash.block_hash, HASH_PADDING);
                let tx: Vec<StoredTransactionReceipt> = self.database.get(filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
        }
    }

    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>> {
        let block_id = match block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)) {
            BlockId::Number(maybe_number) => self.tag_into_block_number(maybe_number).await?.to::<u64>().into(),
            BlockId::Hash(hash) => hash.block_hash.into(),
        };
        if !self.block_exists(block_id).await? {
            return Ok(None);
        }

        match self.transactions(block_id, true).await? {
            BlockTransactions::Full(transactions) => Ok(Some(transactions)),
            _ => Err(TransactionError::ExpectedFullTransactions.into()),
        }
    }
}

impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    pub async fn new(database: Database, starknet_provider: SP) -> Result<Self> {
        let chain_id = starknet_provider.chain_id().await?;
        // We take the chain_id modulo u32::MAX to ensure compatibility with tooling
        // see: https://github.com/ethereum/EIPs/issues/2294
        // Note: Metamask is breaking for a chain_id = u64::MAX - 1
        let chain_id = (FieldElement::from(u32::MAX) & chain_id).try_into().unwrap(); // safe unwrap
        Ok(Self { database, starknet_provider, chain_id })
    }

    #[cfg(feature = "testing")]
    pub fn starknet_provider(&self) -> &SP {
        &self.starknet_provider
    }

    /// Prepare the call input for an estimate gas or call from a transaction request.
    async fn prepare_call_input(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<CallInput> {
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

        let gas_limit = into_via_try_wrapper!(request.gas.unwrap_or_else(|| U256::from(CALL_REQUEST_GAS_LIMIT)))?;

        // We cannot unwrap_or_default() here because Kakarot.eth_call will
        // Reject transactions with gas_price < Kakarot.base_fee
        let gas_price = {
            let gas_price = match request.gas_price {
                Some(gas_price) => gas_price,
                None => self.gas_price().await?,
            };
            into_via_try_wrapper!(gas_price)?
        };

        let value =
            Uint256 { low: into_via_try_wrapper!(request.value.unwrap_or_default())?, high: FieldElement::ZERO };

        // TODO: replace this by into_via_wrapper!(request.nonce.unwrap_or_default())
        //  when we can simulate the transaction instead of calling `eth_call`
        let nonce = {
            match request.nonce {
                Some(nonce) => into_via_wrapper!(nonce),
                None => match request.from {
                    None => FieldElement::ZERO,
                    Some(address) => into_via_try_wrapper!(self.transaction_count(address, block_id).await?)?,
                },
            }
        };

        Ok(CallInput { nonce, from, to, gas_limit, gas_price, value, calldata })
    }

    /// Call the Kakarot contract with the given request.
    async fn call_helper(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<CairoArrayLegacy<FieldElement>> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;
        let call_input = self.prepare_call_input(request, block_id).await?;

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let call_output = kakarot_contract
            .eth_call(
                &call_input.nonce,
                &call_input.from,
                &call_input.to,
                &call_input.gas_limit,
                &call_input.gas_price,
                &call_input.value,
                &call_input.calldata.len().into(),
                &CairoArrayLegacy(call_input.calldata),
                &FieldElement::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .await
            .map_err(KakarotError::from)?;

        let return_data = call_output.return_data;
        if call_output.success == FieldElement::ZERO {
            return Err(KakarotError::from(EvmError::from(return_data.0)).into());
        }
        Ok(return_data)
    }

    /// Estimate the gas used in Kakarot for the given request.
    async fn estimate_gas_helper(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<u128> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;
        let call_input = self.prepare_call_input(request, block_id).await?;

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let estimate_gas_output = kakarot_contract
            .eth_estimate_gas(
                &call_input.nonce,
                &call_input.from,
                &call_input.to,
                &call_input.gas_limit,
                &call_input.gas_price,
                &call_input.value,
                &call_input.calldata.len().into(),
                &CairoArrayLegacy(call_input.calldata),
                &FieldElement::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .await
            .map_err(KakarotError::from)?;

        let return_data = estimate_gas_output.return_data;
        if estimate_gas_output.success == FieldElement::ZERO {
            return Err(KakarotError::from(EvmError::from(return_data.0)).into());
        }
        let required_gas = estimate_gas_output.required_gas.try_into().map_err(|_| TransactionError::GasOverflow)?;
        Ok(required_gas)
    }

    /// Check if a block exists in the database.
    async fn block_exists(&self, block_id: BlockHashOrNumber) -> EthProviderResult<bool> {
        Ok(self.header(block_id).await?.is_some())
    }

    /// Get a header from the database based on the filter.
    async fn header(&self, id: BlockHashOrNumber) -> EthProviderResult<Option<StoredHeader>> {
        let filter = match id {
            BlockHashOrNumber::Hash(hash) => into_filter("header.hash", &hash, HASH_PADDING),
            BlockHashOrNumber::Number(number) => into_filter("header.number", &number, U64_PADDING),
        };
        self.database
            .get_one(filter, None)
            .await
            .inspect_err(|err| {
                tracing::error!("internal error: {:?}", err);
            })
            .map_err(|_| EthApiError::UnknownBlock)
    }

    /// Return the transactions given a block id.
    pub(crate) async fn transactions(
        &self,
        block_id: BlockHashOrNumber,
        full: bool,
    ) -> EthProviderResult<BlockTransactions> {
        let transactions_filter = match block_id {
            BlockHashOrNumber::Hash(hash) => into_filter("tx.blockHash", &hash, HASH_PADDING),
            BlockHashOrNumber::Number(number) => into_filter("tx.blockNumber", &number, U64_PADDING),
        };

        let block_transactions = if full {
            BlockTransactions::Full(
                self.database.get_and_map_to::<_, StoredTransaction>(transactions_filter, None).await?,
            )
        } else {
            BlockTransactions::Hashes(
                self.database
                    .get_and_map_to::<_, StoredTransactionHash>(transactions_filter, doc! {"tx.hash": 1})
                    .await?,
            )
        };

        Ok(block_transactions)
    }

    /// Get a block from the database based on a block hash or number.
    /// If full is true, the block will contain the full transactions, otherwise just the hashes
    async fn block(&self, block_id: BlockHashOrNumber, full: bool) -> EthProviderResult<Option<RichBlock>> {
        let header = match self.header(block_id).await? {
            Some(h) => h.header,
            None => return Ok(None),
        };

        // The withdrawals are not supported, hence the withdrawals_root should always be empty.
        if let Some(withdrawals_root) = header.withdrawals_root {
            if withdrawals_root != EMPTY_ROOT_HASH {
                return Err(EthApiError::Unsupported("withdrawals"));
            }
        }

        // This is how reth computes the block size.
        // `https://github.com/paradigmxyz/reth/blob/v0.2.0-beta.5/crates/rpc/rpc-types-compat/src/block.rs#L66`
        let size = rpc_to_primitive_header(header.clone())?.length();
        Ok(Some(
            Block {
                header,
                transactions: self.transactions(block_id, full).await?,
                uncles: Default::default(),
                size: Some(U256::from(size)),
                withdrawals: Some(Default::default()),
                other: Default::default(),
            }
            .into(),
        ))
    }

    /// Convert the given block id into a Starknet block id
    pub async fn to_starknet_block_id(
        &self,
        block_id: impl Into<Option<BlockId>>,
    ) -> EthProviderResult<starknet::core::types::BlockId> {
        match block_id.into() {
            Some(BlockId::Hash(hash)) => {
                Ok(EthBlockId::new(BlockId::Hash(hash)).try_into().map_err(EthereumDataFormatError::from)?)
            }
            Some(BlockId::Number(number_or_tag)) => {
                // There is a need to separate the BlockNumberOrTag case into three subcases
                // because pending Starknet blocks don't have a number.
                // 1. The block number corresponds to a Starknet pending block, then we return the pending tag
                // 2. The block number corresponds to a Starknet sealed block, then we return the block number
                // 3. The block number is not found, then we return an error
                match number_or_tag {
                    BlockNumberOrTag::Number(number) => {
                        let header = self.header(number.into()).await?.ok_or(EthApiError::UnknownBlockNumber)?;
                        // If the block hash is zero, then the block corresponds to a Starknet pending block
                        if header.header.hash.ok_or(EthApiError::UnknownBlock)?.is_zero() {
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

    /// Converts the given [`BlockNumberOrTag`] into a block number.
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        match tag {
            // Converts the tag representing the earliest block into block number 0.
            BlockNumberOrTag::Earliest => Ok(U64::ZERO),
            // Converts the tag containing a specific block number into a `U64`.
            BlockNumberOrTag::Number(number) => Ok(U64::from(number)),
            // Returns `self.block_number()` which is the block number of the latest finalized block.
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await
            }
            // Adds 1 to the block number of the latest finalized block.
            BlockNumberOrTag::Pending => Ok(self.block_number().await?.saturating_add(U64::from(1))),
        }
    }
}

#[cfg(feature = "hive")]
impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    /// Deploy the EVM transaction signer if a corresponding contract is not found on
    /// Starknet.
    async fn deploy_evm_transaction_signer(&self, signer: Address) -> EthProviderResult<()> {
        use crate::eth_provider::constant::{DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
        use starknet::accounts::{Call, Execution};
        use starknet::core::types::BlockTag;

        let signer_starknet_address = starknet_address(signer);
        let account_contract = AccountContractReader::new(signer_starknet_address, &self.starknet_provider);
        let maybe_is_initialized = account_contract
            .is_initialized()
            .block_id(starknet::core::types::BlockId::Tag(BlockTag::Latest))
            .call()
            .await;

        if contract_not_found(&maybe_is_initialized) {
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
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversionError))?
                .get_invoke_request(false)
                .await
                .map_err(|_| SignatureError::SignError)?;
            self.starknet_provider.add_invoke_transaction(tx).await.map_err(KakarotError::from)?;

            *nonce += 1u8.into();
            drop(nonce);
        };

        Ok(())
    }
}
