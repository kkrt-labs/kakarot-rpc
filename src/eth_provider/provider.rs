use alloy_rlp::Decodable as _;
use async_trait::async_trait;
use auto_impl::auto_impl;
use cainome::cairo_serde::CairoArrayLegacy;
use eyre::Result;
use mongodb::bson::doc;
use mongodb::bson::Document;
use reth_primitives::Address;
use reth_primitives::BlockId;
use reth_primitives::Bytes;
use reth_primitives::TransactionSigned;
use reth_primitives::{BlockNumberOrTag, B256, U256, U64};
use reth_rpc_types::other::OtherFields;
use reth_rpc_types::FeeHistory;
use reth_rpc_types::Filter;
use reth_rpc_types::FilterChanges;
use reth_rpc_types::Index;
use reth_rpc_types::Transaction as RpcTransaction;
use reth_rpc_types::TransactionReceipt;
use reth_rpc_types::TransactionRequest;
use reth_rpc_types::ValueOrArray;
use reth_rpc_types::{Block, BlockTransactions, RichBlock};
use reth_rpc_types::{SyncInfo, SyncStatus};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::core::types::SyncStatusType;
use starknet::core::types::ValueOutOfRangeError;
use starknet::core::utils::get_storage_var_address;
use starknet::providers::Provider as StarknetProvider;
use starknet_crypto::FieldElement;

use super::constant::MAX_FEE;
use super::database::types::log::StoredLog;
use super::database::types::{
    header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
    transaction::StoredTransactionHash,
};
use super::database::Database;
use super::starknet::kakarot_core;
use super::starknet::kakarot_core::core::{KakarotCoreReader, Uint256};
use super::starknet::kakarot_core::to_starknet_transaction;
use super::starknet::kakarot_core::{
    contract_account::ContractAccountReader, proxy::ProxyReader, starknet_address, CONTRACT_ACCOUNT_CLASS_HASH,
    EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH, KAKAROT_ADDRESS,
};
use super::starknet::ERC20Reader;
use super::starknet::STARKNET_NATIVE_TOKEN;
use super::utils::contract_not_found;
use super::utils::iter_into;
use super::utils::split_u256;
use super::utils::try_from_u8_iterator;
use super::{error::EthProviderError, utils::into_filter};
use crate::eth_provider::utils::format_hex;
use crate::into_via_try_wrapper;
use crate::into_via_wrapper;
use crate::models::block::EthBlockId;
use crate::models::errors::ConversionError;
use crate::models::felt::Felt252Wrapper;

pub type EthProviderResult<T> = Result<T, EthProviderError>;

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
    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<U64>;
    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64>;
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<RpcTransaction>>;
    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<RpcTransaction>>;
    /// Returns the transaction by block number and index.
    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<RpcTransaction>>;
    /// Returns the transaction receipt by hash of the transaction.
    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>>;
    /// Returns the balance of an address in native eth.
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the storage of an address at a certain index.
    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> EthProviderResult<U256>;
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
        block_count: U256,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory>;
    /// Send a raw transaction to the network and returns the transactions hash.
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256>;
}

/// Structure that implements the EthereumProvider trait.
/// Uses an access to a database to certain data, while
/// the rest is fetched from the Starknet Provider.
pub struct EthDataProvider<SP: StarknetProvider> {
    database: Database,
    starknet_provider: SP,
}

#[async_trait]
impl<SP> EthereumProvider for EthDataProvider<SP>
where
    SP: StarknetProvider + Send + Sync,
{
    async fn block_number(&self) -> EthProviderResult<U64> {
        let filter = doc! {};
        let sort = doc! { "header.number": -1 };
        let header: Option<StoredHeader> = self.database.get_one("headers", filter, sort).await?;
        let block_number = match header {
            None => U64::from(self.starknet_provider.block_number().await?), // in case the database is empty, use the starknet provider
            Some(header) => {
                let number = header.header.number.ok_or(EthProviderError::ValueNotFound)?;
                let n = number.as_le_bytes_trimmed();
                // Block number is U64
                if n.len() > 8 {
                    return Err(ConversionError::ValueOutOfRange("Block number too large".to_string()).into());
                }
                U64::from_le_slice(n.as_ref())
            }
        };
        Ok(block_number)
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        let syncing_status = self.starknet_provider.syncing().await?;

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
        let chain_id = self.starknet_provider.chain_id().await?;
        let chain_id: Option<u64> = chain_id.try_into().ok();
        Ok(chain_id.map(U64::from))
    }

    async fn block_by_hash(&self, hash: B256, full: bool) -> EthProviderResult<Option<RichBlock>> {
        let header_filter = into_filter("header.hash", hash, 64);
        let tx_filter = into_filter("tx.blockHash", hash, 64);
        let block = self.block(header_filter, tx_filter, full).await?;

        Ok(block)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;

        let header_filter = into_filter("header.number", block_number, 64);
        let tx_filter = into_filter("tx.blockNumber", block_number, 64);
        let block = self.block(header_filter, tx_filter, full).await?;

        Ok(block)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<U64> {
        let filter = into_filter("tx.blockHash", hash, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(U64::from(count))
    }

    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;

        let filter = into_filter("tx.blockNumber", block_number, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(U64::from(count))
    }

    async fn transaction_by_hash(&self, hash: B256) -> EthProviderResult<Option<RpcTransaction>> {
        let filter = into_filter("tx.hash", hash, 64);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthProviderResult<Option<RpcTransaction>> {
        let mut filter = into_filter("tx.blockHash", hash, 64);
        let index: usize = index.into();
        filter.insert("tx.transactionIndex", index as i32);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<RpcTransaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let mut filter = into_filter("tx.blockNumber", block_number, 64);
        let index: usize = index.into();
        filter.insert("tx.transactionIndex", index as i32);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        let filter = into_filter("receipt.transactionHash", hash, 64);
        let tx: Option<StoredTransactionReceipt> = self.database.get_one("receipts", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, &self.starknet_provider);

        let address = starknet_address(address);
        let balance = eth_contract.balanceOf(&address).block_id(starknet_block_id).call().await?.balance;

        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);
        Ok(low + (high << 128))
    }

    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);

        let keys = split_u256::<FieldElement>(index);
        let storage_address = get_storage_var_address("storage_", &keys).expect("Storage var name is not ASCII");

        let storage = contract.storage(&storage_address).block_id(starknet_block_id).call().await?.value;

        let low: U256 = into_via_wrapper!(storage.low);
        let high: U256 = into_via_wrapper!(storage.high);
        Ok(low + (high << 128))
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = starknet_address(address);
        let proxy = ProxyReader::new(address, &self.starknet_provider);
        let maybe_class_hash = proxy.get_implementation().block_id(starknet_block_id).call().await;

        if contract_not_found(&maybe_class_hash) {
            return Ok(U256::ZERO);
        }
        let class_hash = maybe_class_hash?.implementation;

        let nonce = if class_hash == *EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH {
            self.starknet_provider.get_nonce(starknet_block_id, address).await?
        } else if class_hash == *CONTRACT_ACCOUNT_CLASS_HASH {
            let contract = ContractAccountReader::new(address, &self.starknet_provider);
            contract.get_nonce().block_id(starknet_block_id).call().await?.nonce
        } else {
            FieldElement::ZERO
        };
        Ok(into_via_wrapper!(nonce))
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);
        let bytecode = contract.bytecode().block_id(starknet_block_id).call().await?.bytecode;

        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<u8>>(bytecode.0.into_iter())))
    }

    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        let current_block = self.block_number().await?.try_into().map_err(ConversionError::from)?;
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
        Ok(FilterChanges::Logs(logs.into_iter().map(Into::into).collect()))
    }

    async fn call(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let (output, _) = self.call_helper(request, block_id).await?;
        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<_>>(output.0.into_iter())))
    }

    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        // Set a high gas limit to make sure the transaction will not fail due to gas.
        let request = TransactionRequest { gas: Some(U256::from(*MAX_FEE)), ..request };

        let (_, gas_used) = self.call_helper(request, block_id).await?;
        Ok(U256::from(gas_used))
    }

    async fn fee_history(
        &self,
        mut block_count: U256,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory> {
        if block_count == U256::ZERO {
            return Ok(FeeHistory::default());
        }

        let end_block = U256::from(self.tag_into_block_number(newest_block).await?);
        let end_block_plus = end_block + U256::from(1);

        // If the block count is larger than the end block, we need to reduce it.
        if end_block_plus < block_count {
            block_count = end_block_plus;
        }
        let start_block = end_block_plus - block_count;

        let bc = usize::try_from(block_count).map_err(|e| ConversionError::ValueOutOfRange(e.to_string()))?;
        // We add one to the block count and fill with 0's.
        // This comes from the rpc spec: `An array of block base fees per gas.
        // This includes the next block after the newest of the returned range,
        // because this value can be derived from the newest block. Zeroes are returned for pre-EIP-1559 blocks.`
        // Since Kakarot doesn't support EIP-1559 yet, we just fill with 0's.
        let base_fee_per_gas = vec![U256::ZERO; bc + 1];

        // TODO: check if we should use a projection since we only need the gasLimit and gasUsed.
        // This means we need to introduce a new type for the StoredHeader.
        let header_filter =
            doc! {"header.number": {"$gte": format_hex(start_block, 64), "$lte": format_hex(end_block, 64)}};
        let blocks: Vec<StoredHeader> = self.database.get("headers", header_filter, None).await?;
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

        Ok(FeeHistory {
            base_fee_per_gas,
            gas_used_ratio,
            oldest_block: start_block,
            reward: Some(vec![]),
            ..Default::default()
        })
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256> {
        let mut data = transaction.0.as_ref();
        let transaction_signed = TransactionSigned::decode(&mut data)
            .map_err(|err| ConversionError::ToStarknetTransactionError(err.to_string()))?;

        let chain_id = self.chain_id().await?.unwrap_or_default().try_into().map_err(ConversionError::from)?;

        let signer = transaction_signed
            .recover_signer()
            .ok_or_else(|| ConversionError::ToStarknetTransactionError("Failed to recover signer".to_string()))?;
        let transaction = to_starknet_transaction(&transaction_signed, chain_id, signer)?;

        #[cfg(not(feature = "testing"))]
        {
            let hash = transaction_signed.hash();
            self.starknet_provider.add_invoke_transaction(transaction).await?;
            Ok(hash)
        }
        // If we are currently testing, we need to return the starknet hash in order
        // to be able to wait for the transaction to be mined.
        #[cfg(feature = "testing")]
        {
            let res = self.starknet_provider.add_invoke_transaction(transaction).await?;
            Ok(B256::from_slice(&res.transaction_hash.to_bytes_be()[..]))
        }
    }
}

impl<SP> EthDataProvider<SP>
where
    SP: StarknetProvider + Send + Sync,
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
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

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
        let calldata: Vec<_> = data.into_iter().map(FieldElement::from).collect();

        let gas_limit = into_via_try_wrapper!(request.gas.unwrap_or_else(|| U256::from(u64::MAX)));
        let gas_price = into_via_try_wrapper!(request.gas_price.unwrap_or_default());

        let value = into_via_try_wrapper!(request.value.unwrap_or_default());

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let call_output = kakarot_contract
            .eth_call(
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
            .await?;

        let return_data = call_output.return_data;
        if call_output.success == FieldElement::ZERO {
            let revert_reason =
                return_data.0.into_iter().filter_map(|x| u8::try_from(x).ok()).map(|x| x as char).collect::<String>();
            return Err(EthProviderError::EvmExecutionError(revert_reason));
        }
        let gas_used = call_output
            .gas_used
            .try_into()
            .map_err(|err: ValueOutOfRangeError| ConversionError::ValueOutOfRange(err.to_string()))?;
        Ok((return_data, gas_used))
    }

    /// Get a block from the database based on the header and transaction filters
    /// If full is true, the block will contain the full transactions, otherwise just the hashes
    async fn block(
        &self,
        header_filter: impl Into<Option<Document>>,
        transactions_filter: impl Into<Option<Document>>,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let header = self.database.get_one::<StoredHeader>("headers", header_filter, None).await?;
        let header = match header {
            Some(header) => header,
            None => return Ok(None),
        };
        let total_difficulty = Some(header.header.difficulty);

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

        let block = Block {
            header: header.header,
            transactions,
            total_difficulty,
            uncles: Vec::new(),
            size: None,
            withdrawals: None,
            other: OtherFields::default(),
        };

        Ok(Some(block.into()))
    }

    /// Convert the given BlockNumberOrTag into a block number
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        match tag {
            BlockNumberOrTag::Earliest => Ok(U64::ZERO),
            BlockNumberOrTag::Number(number) => Ok(U64::from(number)),
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await
            }
            BlockNumberOrTag::Pending => todo!("pending block number not implemented"),
        }
    }
}
