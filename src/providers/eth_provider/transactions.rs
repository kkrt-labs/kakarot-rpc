use super::{
    database::{filter::EthDatabaseFilterBuilder, types::transaction::StoredTransaction},
    error::ExecutionError,
    starknet::kakarot_core::{account_contract::AccountContractReader, starknet_address},
    utils::{contract_not_found, entrypoint_not_found},
};
use crate::{
    into_via_wrapper,
    providers::eth_provider::{
        database::filter::{self},
        provider::{EthApiResult, EthDataProvider},
        ChainProvider,
    },
};
use alloy_primitives::{Address, B256, U256};
use alloy_rpc_types::{Index, Transaction};
use alloy_serde::WithOtherFields;
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{BlockId, BlockNumberOrTag};
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait TransactionProvider: ChainProvider {
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<WithOtherFields<Transaction>>>;

    /// Returns the transaction by block hash and index.
    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthApiResult<Option<WithOtherFields<Transaction>>>;

    /// Returns the transaction by block number and index.
    async fn transaction_by_block_number_and_index(
        &self,
        number_or_tag: BlockNumberOrTag,
        index: Index,
    ) -> EthApiResult<Option<WithOtherFields<Transaction>>>;

    /// Returns the nonce for the address at the given block.
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256>;
}

#[async_trait]
impl<SP> TransactionProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<WithOtherFields<Transaction>>> {
        let filter = EthDatabaseFilterBuilder::<filter::Transaction>::default().with_tx_hash(&hash).build();
        Ok(self.database().get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_by_block_hash_and_index(
        &self,
        hash: B256,
        index: Index,
    ) -> EthApiResult<Option<WithOtherFields<Transaction>>> {
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
    ) -> EthApiResult<Option<WithOtherFields<Transaction>>> {
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

        Ok(into_via_wrapper!(nonce))
    }
}
