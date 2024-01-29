use async_trait::async_trait;
use auto_impl::auto_impl;
use eyre::Result;
use mockall::automock;
use mongodb::bson::doc;
use mongodb::bson::Document;
use reth_primitives::Address;
use reth_primitives::BlockId;
use reth_primitives::Bytes;
use reth_primitives::{BlockNumberOrTag, H256, U256, U64};
use reth_rpc_types::Filter;
use reth_rpc_types::FilterChanges;
use reth_rpc_types::Index;
use reth_rpc_types::Transaction;
use reth_rpc_types::TransactionReceipt;
use reth_rpc_types::ValueOrArray;
use reth_rpc_types::{Block, BlockTransactions, RichBlock};
use reth_rpc_types::{SyncInfo, SyncStatus};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::core::types::SyncStatusType;
use starknet::core::utils::get_contract_address;
use starknet::core::utils::get_storage_var_address;
use starknet::providers::Provider as StarknetProvider;
use starknet_crypto::FieldElement;

use super::database::types::log::StoredLog;
use super::database::types::{
    header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
    transaction::StoredTransactionHash,
};
use super::database::Database;
use super::starknet::kakarot::{
    ContractAccountReader, ProxyReader, CONTRACT_ACCOUNT_CLASS_HASH, EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH,
    KAKAROT_ADDRESS, PROXY_ACCOUNT_CLASS_HASH,
};
use super::starknet::ERC20Reader;
use super::starknet::STARKNET_NATIVE_TOKEN;
use super::utils::iter_into;
use super::utils::split_u256;
use super::utils::try_from_u8_iterator;
use super::{error::EthProviderError, utils::into_filter};
use crate::eth_provider::utils::format_hex;
use crate::into_via_wrapper;
use crate::models::block::EthBlockId;
use crate::models::errors::ConversionError;
use crate::models::felt::Felt252Wrapper;

pub type EthProviderResult<T> = Result<T, EthProviderError>;

/// Ethereum provider trait. Used to abstract away the database and the network.
#[async_trait]
#[auto_impl(Arc)]
#[automock]
pub trait EthereumProvider {
    /// Returns the latest block number.
    async fn block_number(&self) -> EthProviderResult<U64>;
    /// Returns the syncing status.
    async fn syncing(&self) -> EthProviderResult<SyncStatus>;
    /// Returns the chain id.
    async fn chain_id(&self) -> EthProviderResult<Option<U64>>;
    /// Returns a block by hash. Block can be full or just the hashes of the transactions.
    async fn block_by_hash(&self, hash: H256, full: bool) -> EthProviderResult<Option<RichBlock>>;
    /// Returns a block by number. Block can be full or just the hashes of the transactions.
    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>>;
    /// Returns the transaction count for a block by hash.
    async fn block_transaction_count_by_hash(&self, hash: H256) -> EthProviderResult<U64>;
    /// Returns the transaction count for a block by number.
    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64>;
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: H256) -> EthProviderResult<Option<Transaction>>;
    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> EthProviderResult<Option<Transaction>>;
    /// Returns the transaction by block number and index.
    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<Transaction>>;
    /// Returns the transaction receipt by hash of the transaction.
    async fn transaction_receipt(&self, hash: H256) -> EthProviderResult<Option<TransactionReceipt>>;
    /// Returns the balance of an address.
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the storage of an address at a certain index.
    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the nonce for the address at the given block.
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the code for the address at the given block.
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes>;
    /// Returns the logs for the given filter
    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges>;
}

/// Structure that implements the EthereumProvider trait.
/// Uses an access to a database to certain data, while
/// the rest is fetched from the Starknet Provider.
pub struct EthDataProvider<SP>
where
    SP: StarknetProvider + Send + Sync,
{
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
            None => self.starknet_provider.block_number().await?.into(), // in case the database is empty, use the starknet provider
            Some(header) => {
                let number = header.header.number.ok_or(EthProviderError::ValueNotFound)?;
                let n = number.as_le_bytes_trimmed();
                // Block number is U64
                if n.len() > 8 {
                    return Err(ConversionError::ValueOutOfRange("Block number too large".to_string()).into());
                }
                U64::from_little_endian(n.as_ref())
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

    async fn chain_id(&self) -> EthProviderResult<Option<U64>> {
        let chain_id = self.starknet_provider.chain_id().await?;
        let chain_id: Option<u64> = chain_id.try_into().ok();
        Ok(chain_id.map(Into::into))
    }

    async fn block_by_hash(&self, hash: H256, full: bool) -> EthProviderResult<Option<RichBlock>> {
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

    async fn block_transaction_count_by_hash(&self, hash: H256) -> EthProviderResult<U64> {
        let filter = into_filter("tx.blockHash", hash, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(count.into())
    }

    async fn block_transaction_count_by_number(&self, number_or_tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;

        let filter = into_filter("tx.blockNumber", block_number, 64);
        let count = self.database.count("transactions", filter).await?;
        Ok(count.into())
    }

    async fn transaction_by_hash(&self, hash: H256) -> EthProviderResult<Option<Transaction>> {
        let filter = into_filter("tx.hash", hash, 64);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: H256,
        index: Index,
    ) -> EthProviderResult<Option<Transaction>> {
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
    ) -> EthProviderResult<Option<Transaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let mut filter = into_filter("tx.blockNumber", block_number, 64);
        let index: usize = index.into();
        filter.insert("tx.transactionIndex", index as i32);
        let tx: Option<StoredTransaction> = self.database.get_one("transactions", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn transaction_receipt(&self, hash: H256) -> EthProviderResult<Option<TransactionReceipt>> {
        let filter = into_filter("receipt.transactionHash", hash, 64);
        let tx: Option<StoredTransactionReceipt> = self.database.get_one("receipts", filter, None).await?;
        Ok(tx.map(Into::into))
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, &self.starknet_provider);

        let address = self.starknet_address(address);
        let balance = eth_contract.balanceOf(&address).block_id(starknet_block_id).call().await?;

        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);
        Ok(low + (high << 128))
    }

    async fn storage_at(&self, address: Address, index: U256, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = self.starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);

        let keys = split_u256::<FieldElement>(index);
        let storage_address = get_storage_var_address("storage_", &keys).expect("Storage var name is not ASCII");

        let storage = contract.storage(&storage_address).block_id(starknet_block_id).call().await?;

        let low: U256 = into_via_wrapper!(storage.low);
        let high: U256 = into_via_wrapper!(storage.high);
        Ok(low + (high << 128))
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = self.starknet_address(address);
        let proxy = ProxyReader::new(address, &self.starknet_provider);
        let address_class_hash = proxy.get_implementation().block_id(starknet_block_id).call().await?;

        let nonce = if address_class_hash == *EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH {
            self.starknet_provider.get_nonce(starknet_block_id, address).await?
        } else if address_class_hash == *CONTRACT_ACCOUNT_CLASS_HASH {
            let contract = ContractAccountReader::new(address, &self.starknet_provider);
            contract.get_nonce().block_id(starknet_block_id).call().await?
        } else {
            FieldElement::ZERO
        };
        Ok(into_via_wrapper!(nonce))
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let eth_block_id = EthBlockId::new(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)));
        let starknet_block_id: StarknetBlockId = eth_block_id.try_into()?;

        let address = self.starknet_address(address);
        let contract = ContractAccountReader::new(address, &self.starknet_provider);
        let (_, bytecode) = contract.bytecode().block_id(starknet_block_id).call().await?;

        Ok(Bytes::from(try_from_u8_iterator::<_, Vec<u8>>(bytecode.0.into_iter())))
    }

    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        let current_block = self.block_number().await?.low_u64();
        let from = filter.get_from_block().unwrap_or_default();
        let to = filter.get_to_block().unwrap_or(current_block);

        let (from, to) = match (from, to) {
            (from, _) if from > current_block => return Ok(FilterChanges::Empty),
            (from, to) if to > current_block => (from, current_block),
            (from, to) if to < from => return Ok(FilterChanges::Empty),
            _ => (from, to),
        };

        // Convert the topics to a vector of H256
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
}

impl<SP> EthDataProvider<SP>
where
    SP: StarknetProvider + Send + Sync,
{
    pub fn new(database: Database, starknet_provider: SP) -> Self {
        Self { database, starknet_provider }
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
        };

        Ok(Some(block.into()))
    }

    /// Convert the given BlockNumberOrTag into a block number
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<U64> {
        match tag {
            BlockNumberOrTag::Earliest => Ok(U64::zero()),
            BlockNumberOrTag::Number(number) => Ok(number.into()),
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await
            }
            BlockNumberOrTag::Pending => todo!("pending block number not implemented"),
        }
    }

    /// Compute the starknet address given a eth address
    fn starknet_address(&self, address: Address) -> FieldElement {
        get_contract_address(into_via_wrapper!(address), *PROXY_ACCOUNT_CLASS_HASH, &[], *KAKAROT_ADDRESS)
    }
}
