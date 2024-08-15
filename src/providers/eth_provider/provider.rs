use super::{
    constant::{BLOCK_NUMBER_HEX_STRING_LEN, CALL_REQUEST_GAS_LIMIT, HASH_HEX_STRING_LEN, MAX_LOGS},
    database::{
        ethereum::EthereumBlockStore,
        filter::EthDatabaseFilterBuilder,
        state::{EthCacheDatabase, EthDatabase},
        types::{
            header::StoredHeader,
            log::StoredLog,
            receipt::StoredTransactionReceipt,
            transaction::{StoredPendingTransaction, StoredTransaction},
        },
        CollectionName, Database,
    },
    error::{
        EthApiError, EthereumDataFormatError, EvmError, ExecutionError, KakarotError, SignatureError, TransactionError,
    },
    starknet::{
        kakarot_core::{
            self,
            account_contract::AccountContractReader,
            core::{CallInput, KakarotCoreReader, Uint256},
            starknet_address, to_starknet_transaction, KAKAROT_ADDRESS,
        },
        ERC20Reader, STARKNET_NATIVE_TOKEN,
    },
    utils::{class_hash_not_declared, contract_not_found, entrypoint_not_found, split_u256},
};
use crate::{
    into_via_try_wrapper, into_via_wrapper,
    models::{
        block::{EthBlockId, EthBlockNumberOrTag},
        felt::Felt252Wrapper,
        transaction::validate_transaction,
    },
    providers::eth_provider::database::{
        ethereum::EthereumTransactionStore,
        filter::{self, format_hex},
        FindOpts,
    },
};
use alloy_rlp::Decodable;
use async_trait::async_trait;
use auto_impl::auto_impl;
use cainome::cairo_serde::CairoArrayLegacy;
use eyre::{eyre, Result};
use itertools::Itertools;
use mongodb::bson::doc;
use num_traits::cast::ToPrimitive;
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::ConfigureEvm;
use reth_primitives::{
    Address, BlockId, BlockNumberOrTag, Bytes, TransactionSigned, TransactionSignedEcRecovered, TxKind, B256, U256, U64,
};
use reth_revm::{
    db::CacheDB,
    primitives::{BlockEnv, CfgEnv, CfgEnvWithHandlerCfg, HandlerCfg, SpecId},
};
use reth_rpc_eth_types::{error::ensure_success, revm_utils::prepare_call_env};
use reth_rpc_types::{
    serde_helpers::JsonStorageKey,
    state::{EvmOverrides, StateOverride},
    txpool::TxpoolContent,
    BlockHashOrNumber, BlockOverrides, FeeHistory, Filter, FilterChanges, Header, Index, RichBlock, SyncInfo,
    SyncStatus, Transaction, TransactionReceipt, TransactionRequest,
};
use reth_rpc_types_compat::transaction::from_recovered;
#[cfg(feature = "hive")]
use starknet::core::types::BroadcastedInvokeTransaction;
use starknet::core::{
    types::{Felt, SyncStatusType},
    utils::get_storage_var_address,
};
use tracing::{instrument, Instrument};

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
    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> EthProviderResult<Bytes>;
    /// Returns the result of a estimate gas.
    async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256>;
    /// Returns the fee history given a block count and a newest block number.
    async fn fee_history(
        &self,
        block_count: U64,
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
    /// Returns a vec of pending pool transactions.
    async fn txpool_transactions(&self) -> EthProviderResult<Vec<Transaction>>;
    /// Returns the content of the pending pool.
    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent>;
}

/// Structure that implements the `EthereumProvider` trait.
/// Uses access to a database for certain data, while
/// the rest is fetched from the Starknet Provider.
#[derive(Debug, Clone)]
pub struct EthDataProvider<SP: starknet::providers::Provider> {
    database: Database,
    starknet_provider: SP,
    chain_id: u64,
}

impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider,
{
    /// Returns a reference to the database.
    pub const fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl<SP> EthereumProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn header(&self, block_id: &BlockId) -> EthProviderResult<Option<Header>> {
        let block_hash_or_number = self.block_id_into_block_number_or_hash(*block_id).await?;
        Ok(self.database.header(block_hash_or_number).await?)
    }

    async fn block_number(&self) -> EthProviderResult<U64> {
        let block_number = match self.database.latest_header().await? {
            // In case the database is empty, use the starknet provider
            None => {
                let span = tracing::span!(tracing::Level::INFO, "sn::block_number");
                U64::from(self.starknet_provider.block_number().instrument(span).await.map_err(KakarotError::from)?)
            }
            Some(header) => {
                let number = header.number.ok_or(EthApiError::UnknownBlockNumber(None))?;
                let is_pending_block = header.hash.unwrap_or_default().is_zero();
                U64::from(if is_pending_block { number - 1 } else { number })
            }
        };
        Ok(block_number)
    }

    async fn syncing(&self) -> EthProviderResult<SyncStatus> {
        let span = tracing::span!(tracing::Level::INFO, "sn::syncing");
        Ok(match self.starknet_provider.syncing().instrument(span).await.map_err(KakarotError::from)? {
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
        Ok(self.database.block(hash.into(), full).await?)
    }

    async fn block_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
        full: bool,
    ) -> EthProviderResult<Option<RichBlock>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        Ok(self.database.block(block_number.into(), full).await?)
    }

    async fn block_transaction_count_by_hash(&self, hash: B256) -> EthProviderResult<Option<U256>> {
        self.database.transaction_count(hash.into()).await
    }

    async fn block_transaction_count_by_number(
        &self,
        number_or_tag: BlockNumberOrTag,
    ) -> EthProviderResult<Option<U256>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        self.database.transaction_count(block_number.into()).await
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
                                "tx.hash": format_hex(hash, HASH_HEX_STRING_LEN)
                            }
                        }
                    ]
                },
            },
            // Only specified hash in the transactions collection
            doc! {
                "$match": {
                    "tx.hash": format_hex(hash, HASH_HEX_STRING_LEN)
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
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash(&hash)
            .with_tx_index(&index)
            .build();
        Ok(self.database.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthProviderResult<Option<reth_rpc_types::Transaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_number(block_number)
            .with_tx_index(&index)
            .build();
        Ok(self.database.get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_receipt(&self, hash: B256) -> EthProviderResult<Option<TransactionReceipt>> {
        let filter = EthDatabaseFilterBuilder::<filter::Receipt>::default().with_tx_hash(&hash).build();
        Ok(self.database.get_one::<StoredTransactionReceipt>(filter, None).await?.map(Into::into))
    }

    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        // Convert the optional Ethereum block ID to a Starknet block ID.
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        // Create a new `ERC20Reader` instance for the Starknet native token
        let eth_contract = ERC20Reader::new(*STARKNET_NATIVE_TOKEN, &self.starknet_provider);

        // Call the `balanceOf` method on the contract for the given address and block ID, awaiting the result
        let span = tracing::span!(tracing::Level::INFO, "sn::balance");
        let res = eth_contract
            .balanceOf(&starknet_address(address))
            .block_id(starknet_block_id)
            .call()
            .instrument(span)
            .await;

        // Check if the contract was not found or the class hash not declared,
        // returning a default balance of 0 if true.
        // The native token contract should be deployed on Kakarot, so this should not happen
        // We want to avoid errors in this case and return a default balance of 0
        if contract_not_found(&res) || class_hash_not_declared(&res) {
            return Ok(Default::default());
        }
        // Otherwise, extract the balance from the result, converting any errors to ExecutionError
        let balance = res.map_err(ExecutionError::from)?.balance;

        // Convert the low and high parts of the balance to U256
        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);

        // Combine the low and high parts to form the final balance and return it
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

        let keys = split_u256(index.0);
        let storage_address = get_storage_var_address("Account_storage", &keys).expect("Storage var name is not ASCII");

        let span = tracing::span!(tracing::Level::INFO, "sn::storage");
        let maybe_storage =
            contract.storage(&storage_address).block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&maybe_storage) || entrypoint_not_found(&maybe_storage) {
            return Ok(U256::ZERO.into());
        }

        let storage = maybe_storage.map_err(ExecutionError::from)?.value;
        let low: U256 = into_via_wrapper!(storage.low);
        let high: U256 = into_via_wrapper!(storage.high);
        let storage: U256 = low + (high << 128);

        Ok(storage.into())
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let account_contract = AccountContractReader::new(address, &self.starknet_provider);
        let span = tracing::span!(tracing::Level::INFO, "sn::kkrt_nonce");
        let maybe_nonce = account_contract.get_nonce().block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&maybe_nonce) || entrypoint_not_found(&maybe_nonce) {
            return Ok(U256::ZERO);
        }
        let nonce = maybe_nonce.map_err(ExecutionError::from)?.nonce;

        // Get the protocol nonce as well, in edge cases where the protocol nonce is higher than the account nonce.
        // This can happen when an underlying Starknet transaction reverts => Account storage changes are reverted,
        // but the protocol nonce is still incremented.
        let span = tracing::span!(tracing::Level::INFO, "sn::protocol_nonce");
        let protocol_nonce =
            self.starknet_provider.get_nonce(starknet_block_id, address).instrument(span).await.unwrap_or_default();
        let nonce = nonce.max(protocol_nonce);

        Ok(into_via_wrapper!(nonce))
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<Bytes> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let account_contract = AccountContractReader::new(address, &self.starknet_provider);
        let span = tracing::span!(tracing::Level::INFO, "sn::code");
        let bytecode = account_contract.bytecode().block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&bytecode) || entrypoint_not_found(&bytecode) {
            return Ok(Bytes::default());
        }

        let bytecode = bytecode.map_err(ExecutionError::from)?.bytecode.0;

        Ok(Bytes::from(bytecode.into_iter().filter_map(|x| x.to_u8()).collect::<Vec<_>>()))
    }

    async fn get_logs(&self, filter: Filter) -> EthProviderResult<FilterChanges> {
        let block_hash = filter.get_block_hash();

        // Create the database filter.
        let mut builder = EthDatabaseFilterBuilder::<filter::Log>::default();
        builder = if block_hash.is_some() {
            // We filter by block hash on matching the exact block hash.
            builder.with_block_hash(&block_hash.unwrap())
        } else {
            let current_block = self.block_number().await?;
            let current_block =
                current_block.try_into().map_err(|_| EthApiError::UnknownBlockNumber(Some(current_block.to())))?;

            let from = filter.get_from_block().unwrap_or_default();
            let to = filter.get_to_block().unwrap_or(current_block);

            let (from, to) = match (from, to) {
                (from, to) if from > current_block || to < from => return Ok(FilterChanges::Empty),
                (from, to) if to > current_block => (from, current_block),
                other => other,
            };
            // We filter by block number using $gte and $lte.
            builder.with_block_number_range(from, to)
        };

        // TODO: this will work for now but isn't very efficient. Would need to:
        // 1. Create the bloom filter from the topics
        // 2. Query the database for logs within block range with the bloom filter
        // 3. Filter this reduced set of logs by the topics
        // 4. Limit the number of logs returned

        // Convert the topics to a MongoDB filter and add it to the database filter
        builder = builder.with_topics(&filter.topics);

        // Add the addresses
        builder = builder.with_addresses(&filter.address.into_iter().collect::<Vec<_>>());

        Ok(FilterChanges::Logs(
            self.database
                .get_and_map_to::<_, StoredLog>(
                    builder.build(),
                    (*MAX_LOGS).map(|limit| FindOpts::default().with_limit(limit)),
                )
                .await?,
        ))
    }

    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> EthProviderResult<Bytes> {
        // Create the EVM overrides from the state and block overrides.
        let evm_overrides = EvmOverrides::new(state_overrides, block_overrides);

        // Check if either state_overrides or block_overrides is present.
        if evm_overrides.has_state() || evm_overrides.has_block() {
            // Create the configuration environment with the chain ID.
            let cfg_env = CfgEnv::default().with_chain_id(self.chain_id().await?.unwrap_or_default().to());

            // Retrieve the block header details.
            let Header { number, timestamp, miner, base_fee_per_gas, difficulty, .. } =
                self.header(&block_id.unwrap_or_default()).await?.unwrap_or_default();

            // Create the block environment with the retrieved header details and transaction request.
            let block_env = BlockEnv {
                number: U256::from(number.unwrap_or_default()),
                timestamp: U256::from(timestamp),
                gas_limit: U256::from(request.gas.unwrap_or_default()),
                coinbase: miner,
                basefee: U256::from(base_fee_per_gas.unwrap_or_default()),
                prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes::<32>()[..])),
                ..Default::default()
            };

            // Combine the configuration environment with the handler configuration.
            let cfg_env_with_handler_cfg =
                CfgEnvWithHandlerCfg { cfg_env, handler_cfg: HandlerCfg::new(SpecId::CANCUN) };

            // Create a snapshot of the Ethereum database using the block ID.
            let mut db = EthCacheDatabase(CacheDB::new(EthDatabase::new(self, block_id.unwrap_or_default())));

            // Prepare the call environment with the transaction request, gas limit, and overrides.
            let env = prepare_call_env(
                cfg_env_with_handler_cfg,
                block_env,
                request.clone(),
                request.gas.unwrap_or_default().try_into().expect("Gas limit is too large"),
                &mut db.0,
                evm_overrides,
            )?;

            // Execute the transaction using the configured EVM asynchronously.
            let res = EthEvmConfig::default()
                .evm_with_env(db.0, env)
                .transact()
                .map_err(|err| <TransactionError as Into<EthApiError>>::into(TransactionError::Call(err.into())))?;

            // Ensure the transaction was successful and return the result.
            return Ok(ensure_success(res.result)?);
        }

        // If no state or block overrides are present, call the helper function to execute the call.
        let output = self.call_helper(request, block_id).await?;
        Ok(Bytes::from(output.0.into_iter().filter_map(|x| x.to_u8()).collect::<Vec<_>>()))
    }

    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        // Set a high gas limit to make sure the transaction will not fail due to gas.
        let request = TransactionRequest { gas: Some(u128::from(u64::MAX)), ..request };

        let gas_used = self.estimate_gas_helper(request, block_id).await?;

        // Increase the gas used by 20% to make sure the transaction will not fail due to gas.
        // This is a temporary solution until we have a proper gas estimation.
        // Does not apply to Hive feature otherwise end2end tests will fail.
        let gas_used = if cfg!(feature = "hive") { gas_used } else { gas_used * 120 / 100 };
        Ok(U256::from(gas_used))
    }

    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> EthProviderResult<FeeHistory> {
        if block_count == U64::ZERO {
            return Ok(FeeHistory::default());
        }

        let end_block = self.tag_into_block_number(newest_block).await?;
        let end_block_plus_one = end_block.saturating_add(1);

        // 0 <= start_block <= end_block
        let start_block = end_block_plus_one.saturating_sub(block_count.to());

        let header_filter = doc! {"$and": [ { "header.number": { "$gte": format_hex(start_block, BLOCK_NUMBER_HEX_STRING_LEN) } }, { "header.number": { "$lte": format_hex(end_block, BLOCK_NUMBER_HEX_STRING_LEN) } } ] };
        let blocks: Vec<StoredHeader> = self.database.get(header_filter, None).await?;

        if blocks.is_empty() {
            return Err(
                KakarotError::from(mongodb::error::Error::custom(eyre!("No blocks found in the database"))).into()
            );
        }

        let gas_used_ratio = blocks
            .iter()
            .map(|header| {
                let gas_used = header.gas_used as f64;
                let mut gas_limit = header.gas_limit as f64;
                if gas_limit == 0. {
                    gas_limit = 1.;
                };
                gas_used / gas_limit
            })
            .collect();

        let mut base_fee_per_gas =
            blocks.iter().map(|header| header.base_fee_per_gas.unwrap_or_default()).collect::<Vec<_>>();
        // TODO(EIP1559): Remove this when proper base fee computation: if gas_ratio > 50%, increase base_fee_per_gas
        base_fee_per_gas.extend_from_within((base_fee_per_gas.len() - 1)..);

        Ok(FeeHistory {
            base_fee_per_gas,
            gas_used_ratio,
            oldest_block: start_block,
            reward: Some(vec![]),
            ..Default::default()
        })
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256> {
        // Decode the transaction data
        let transaction_signed = TransactionSigned::decode(&mut transaction.0.as_ref())
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?;

        let chain_id: u64 =
            self.chain_id().await?.unwrap_or_default().try_into().map_err(|_| TransactionError::InvalidChainId)?;

        // Validate the transaction
        let latest_block_header = self.database.latest_header().await?.ok_or(EthApiError::UnknownBlockNumber(None))?;
        validate_transaction(&transaction_signed, chain_id, &latest_block_header)?;

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;

        // Get the number of retries for the transaction
        let retries = self.database.pending_transaction_retries(&transaction_signed.hash).await?;

        // Upsert the transaction as pending in the database
        let transaction =
            from_recovered(TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer));
        self.database.upsert_pending_transaction(transaction, retries).await?;

        // Convert the Ethereum transaction to a Starknet transaction
        let starknet_transaction = to_starknet_transaction(&transaction_signed, signer, retries)?;

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.deploy_evm_transaction_signer(signer).await?;

        // Add the transaction to the Starknet provider
        let span = tracing::span!(tracing::Level::INFO, "sn::add_invoke_transaction");
        let res = self
            .starknet_provider
            .add_invoke_transaction(starknet_transaction)
            .instrument(span)
            .await
            .map_err(KakarotError::from)?;

        // Return transaction hash if testing feature is enabled, otherwise log and return Ethereum hash
        if cfg!(feature = "testing") {
            return Ok(B256::from_slice(&res.transaction_hash.to_bytes_be()[..]));
        }
        let hash = transaction_signed.hash();
        tracing::info!(
            "Fired a transaction: Starknet Hash: {} --- Ethereum Hash: {}",
            B256::from_slice(&res.transaction_hash.to_bytes_be()[..]),
            hash
        );

        Ok(hash)
    }

    async fn gas_price(&self) -> EthProviderResult<U256> {
        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let span = tracing::span!(tracing::Level::INFO, "sn::base_fee");
        let gas_price =
            kakarot_contract.get_base_fee().call().instrument(span).await.map_err(ExecutionError::from)?.base_fee;
        Ok(into_via_wrapper!(gas_price))
    }

    async fn block_receipts(&self, block_id: Option<BlockId>) -> EthProviderResult<Option<Vec<TransactionReceipt>>> {
        match block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)) {
            BlockId::Number(number_or_tag) => {
                let block_number = self.tag_into_block_number(number_or_tag).await?;
                if !self.database.block_exists(block_number.into()).await? {
                    return Ok(None);
                }

                let filter =
                    EthDatabaseFilterBuilder::<filter::Receipt>::default().with_block_number(block_number).build();
                let tx: Vec<StoredTransactionReceipt> = self.database.get(filter, None).await?;
                Ok(Some(tx.into_iter().map(Into::into).collect()))
            }
            BlockId::Hash(hash) => {
                if !self.database.block_exists(hash.block_hash.into()).await? {
                    return Ok(None);
                }
                let filter =
                    EthDatabaseFilterBuilder::<filter::Receipt>::default().with_block_hash(&hash.block_hash).build();
                Ok(Some(self.database.get_and_map_to::<_, StoredTransactionReceipt>(filter, None).await?))
            }
        }
    }

    async fn block_transactions(
        &self,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<Option<Vec<reth_rpc_types::Transaction>>> {
        let block_hash_or_number = self
            .block_id_into_block_number_or_hash(block_id.unwrap_or(BlockId::Number(BlockNumberOrTag::Latest)))
            .await?;
        if !self.database.block_exists(block_hash_or_number).await? {
            return Ok(None);
        }

        Ok(Some(self.database.transactions(block_hash_or_number).await?))
    }

    async fn txpool_transactions(&self) -> EthProviderResult<Vec<Transaction>> {
        let span = tracing::span!(tracing::Level::INFO, "sn::txpool");
        Ok(self.database.get_all_and_map_to::<Transaction, StoredPendingTransaction>().instrument(span).await?)
    }

    async fn txpool_content(&self) -> EthProviderResult<TxpoolContent> {
        Ok(self.txpool_transactions().await?.into_iter().fold(TxpoolContent::default(), |mut content, pending| {
            content.pending.entry(pending.from).or_default().insert(pending.nonce.to_string(), pending);
            content
        }))
    }
}

impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    pub async fn new(database: Database, starknet_provider: SP) -> Result<Self> {
        // We take the chain_id modulo u32::MAX to ensure compatibility with tooling
        // see: https://github.com/ethereum/EIPs/issues/2294
        // Note: Metamask is breaking for a chain_id = u64::MAX - 1
        let chain_id =
            (Felt::from(u32::MAX).to_biguint() & starknet_provider.chain_id().await?.to_biguint()).try_into().unwrap(); // safe unwrap

        Ok(Self { database, starknet_provider, chain_id })
    }

    #[cfg(feature = "testing")]
    pub const fn starknet_provider(&self) -> &SP {
        &self.starknet_provider
    }

    /// Prepare the call input for an estimate gas or call from a transaction request.
    #[instrument(skip(self, request), name = "prepare_call")]
    async fn prepare_call_input(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthProviderResult<CallInput> {
        // unwrap option
        let to: kakarot_core::core::Option = {
            match request.to {
                Some(TxKind::Call(to)) => {
                    kakarot_core::core::Option { is_some: Felt::ONE, value: into_via_wrapper!(to) }
                }
                _ => kakarot_core::core::Option { is_some: Felt::ZERO, value: Felt::ZERO },
            }
        };

        // Here we check if CallRequest.origin is None, if so, we insert origin = address(0)
        let from = into_via_wrapper!(request.from.unwrap_or_default());

        let data = request.input.into_input().unwrap_or_default();
        let calldata: Vec<Felt> = data.into_iter().map_into().collect();

        let gas_limit = into_via_try_wrapper!(request.gas.unwrap_or(CALL_REQUEST_GAS_LIMIT))?;

        // We cannot unwrap_or_default() here because Kakarot.eth_call will
        // Reject transactions with gas_price < Kakarot.base_fee
        let gas_price = {
            let gas_price = match request.gas_price {
                Some(gas_price) => U256::from(gas_price),
                None => self.gas_price().await?,
            };
            into_via_try_wrapper!(gas_price)?
        };

        let value = Uint256 { low: into_via_try_wrapper!(request.value.unwrap_or_default())?, high: Felt::ZERO };

        // TODO: replace this by into_via_wrapper!(request.nonce.unwrap_or_default())
        //  when we can simulate the transaction instead of calling `eth_call`
        let nonce = {
            match request.nonce {
                Some(nonce) => into_via_wrapper!(nonce),
                None => match request.from {
                    None => Felt::ZERO,
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
    ) -> EthProviderResult<CairoArrayLegacy<Felt>> {
        tracing::trace!(?request);

        let starknet_block_id = self.to_starknet_block_id(block_id).await?;
        let call_input = self.prepare_call_input(request, block_id).await?;

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, &self.starknet_provider);
        let span = tracing::span!(tracing::Level::INFO, "sn::eth_call");
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
                &Felt::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .instrument(span)
            .await
            .map_err(ExecutionError::from)?;

        let return_data = call_output.return_data;
        if call_output.success == Felt::ZERO {
            return Err(ExecutionError::from(EvmError::from(return_data.0)).into());
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
        let span = tracing::span!(tracing::Level::INFO, "sn::eth_estimate_gas");
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
                &Felt::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .instrument(span)
            .await
            .map_err(ExecutionError::from)?;

        let return_data = estimate_gas_output.return_data;
        if estimate_gas_output.success == Felt::ZERO {
            return Err(ExecutionError::from(EvmError::from(return_data.0)).into());
        }
        let required_gas = estimate_gas_output.required_gas.to_u128().ok_or(TransactionError::GasOverflow)?;
        Ok(required_gas)
    }

    /// Convert the given block id into a Starknet block id
    #[instrument(skip_all, ret)]
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
                        let header = self
                            .database
                            .header(number.into())
                            .await?
                            .ok_or(EthApiError::UnknownBlockNumber(Some(number)))?;
                        // If the block hash is zero, then the block corresponds to a Starknet pending block
                        if header.hash.ok_or(EthApiError::UnknownBlock(number.into()))?.is_zero() {
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
    #[instrument(skip(self))]
    async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthProviderResult<u64> {
        match tag {
            // Converts the tag representing the earliest block into block number 0.
            BlockNumberOrTag::Earliest => Ok(0),
            // Converts the tag containing a specific block number into a `U64`.
            BlockNumberOrTag::Number(number) => Ok(number),
            // Returns `self.block_number()` which is the block number of the latest finalized block.
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await.map(|x| x.to())
            }
            // Adds 1 to the block number of the latest finalized block.
            BlockNumberOrTag::Pending => Ok(self.block_number().await?.to::<u64>().saturating_add(1)),
        }
    }

    /// Converts the given [`BlockId`] into a [`BlockHashOrNumber`].
    #[instrument(skip_all, ret)]
    async fn block_id_into_block_number_or_hash(&self, block_id: BlockId) -> EthProviderResult<BlockHashOrNumber> {
        match block_id {
            BlockId::Hash(hash) => Ok(BlockHashOrNumber::Hash(hash.into())),
            BlockId::Number(number_or_tag) => Ok(self.tag_into_block_number(number_or_tag).await?.into()),
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
        use crate::providers::eth_provider::constant::{DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
        use starknet::{
            accounts::{Call, ExecutionV1},
            core::{types::BlockTag, utils::get_selector_from_name},
        };

        let signer_starknet_address = starknet_address(signer);
        let account_contract = AccountContractReader::new(signer_starknet_address, &self.starknet_provider);
        let maybe_is_initialized = account_contract
            .is_initialized()
            .block_id(starknet::core::types::BlockId::Tag(BlockTag::Latest))
            .call()
            .await;

        if contract_not_found(&maybe_is_initialized) {
            let execution = ExecutionV1::new(
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
                .max_fee(u64::MAX.into())
                .prepared()
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?
                .get_invoke_request(false)
                .await
                .map_err(|_| SignatureError::SigningFailure)?;
            self.starknet_provider
                .add_invoke_transaction(BroadcastedInvokeTransaction::V1(tx))
                .await
                .map_err(KakarotError::from)?;

            *nonce += Felt::ONE;
            drop(nonce);
        };

        Ok(())
    }
}
