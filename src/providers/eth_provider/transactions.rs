use super::{
    constant::HASH_HEX_STRING_LEN,
    database::{filter::EthDatabaseFilterBuilder, types::transaction::StoredTransaction, CollectionName},
    error::ExecutionError,
    starknet::kakarot_core::{account_contract::AccountContractReader, starknet_address},
    utils::{contract_not_found, entrypoint_not_found},
};
use crate::{
    into_via_wrapper,
    providers::eth_provider::{
        database::filter::{self, format_hex},
        provider::{EthApiResult, EthDataProvider},
        ChainProvider,
    },
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, B256, U256};
use reth_rpc_types::Index;
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait TransactionProvider: ChainProvider {
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

    /// Returns the transaction by block number and index.
    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthApiResult<Option<reth_rpc_types::Transaction>>;

    /// Returns the nonce for the address at the given block.
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256>;
}

#[async_trait]
impl<SP> TransactionProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<reth_rpc_types::Transaction>> {
        // TODO: modify this for the tests to pass because now we don't have a pending transactions collection anymore.
        // TODO: So we need to remove the unionWith part and we need to search inside the final transactions collection + inside the mempool.
        let pipeline = vec![
            doc! {
                // Union with pending transactions with only specified hash
                "$unionWith": {
                    "coll": StoredTransaction::collection_name(),
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

        Ok(self.database().get_one_aggregate::<StoredTransaction>(pipeline).await?.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthApiResult<Option<reth_rpc_types::Transaction>> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_hash(&hash)
            .with_tx_index(&index)
            .build();
        Ok(self.database().get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthApiResult<Option<reth_rpc_types::Transaction>> {
        let block_number = self.tag_into_block_number(number_or_tag).await?;
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default()
            .with_block_number(block_number)
            .with_tx_index(&index)
            .build();
        Ok(self.database().get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let account_contract = AccountContractReader::new(address, self.starknet_provider_inner());
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
        let protocol_nonce = self
            .starknet_provider_inner()
            .get_nonce(starknet_block_id, address)
            .instrument(span)
            .await
            .unwrap_or_default();
        let nonce = nonce.max(protocol_nonce);

        Ok(into_via_wrapper!(nonce))
    }
}
