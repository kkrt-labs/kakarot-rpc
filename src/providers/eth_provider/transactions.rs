use super::{
    constant::HASH_HEX_STRING_LEN,
    database::{
        ethereum::EthereumBlockStore,
        filter::EthDatabaseFilterBuilder,
        types::transaction::{StoredPendingTransaction, StoredTransaction},
        CollectionName,
    },
    error::{EthApiError, EthereumDataFormatError, ExecutionError, KakarotError, SignatureError, TransactionError},
    starknet::kakarot_core::{account_contract::AccountContractReader, starknet_address, to_starknet_transaction},
    utils::{contract_not_found, entrypoint_not_found},
};
use crate::{
    into_via_wrapper,
    models::{felt::Felt252Wrapper, transaction::validate_transaction},
    providers::eth_provider::{
        database::{
            ethereum::EthereumTransactionStore,
            filter::{self, format_hex},
        },
        provider::{EthDataProvider, EthProviderResult},
        ChainProvider,
    },
};
use alloy_rlp::Decodable;
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use reth_primitives::{
    Address, BlockId, BlockNumberOrTag, Bytes, TransactionSigned, TransactionSignedEcRecovered, B256, U256,
};
use reth_rpc_types::Index;
use reth_rpc_types_compat::transaction::from_recovered;
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait TransactionProvider: ChainProvider {
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

    /// Returns the nonce for the address at the given block.
    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256>;

    /// Send a raw transaction to the network and returns the transactions hash.
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256>;
}

#[async_trait]
impl<SP> TransactionProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
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

        Ok(self.database().get_one_aggregate::<StoredTransaction>(pipeline).await?.map(Into::into))
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
        Ok(self.database().get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
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
        Ok(self.database().get_one::<StoredTransaction>(filter, None).await?.map(Into::into))
    }

    async fn transaction_count(&self, address: Address, block_id: Option<BlockId>) -> EthProviderResult<U256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let account_contract = AccountContractReader::new(address, self.starknet_provider());
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
            self.starknet_provider().get_nonce(starknet_block_id, address).instrument(span).await.unwrap_or_default();
        let nonce = nonce.max(protocol_nonce);

        Ok(into_via_wrapper!(nonce))
    }

    async fn send_raw_transaction(&self, transaction: Bytes) -> EthProviderResult<B256> {
        // Decode the transaction data
        let transaction_signed = TransactionSigned::decode(&mut transaction.0.as_ref())
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?;

        let chain_id: u64 =
            self.chain_id().await?.unwrap_or_default().try_into().map_err(|_| TransactionError::InvalidChainId)?;

        // Validate the transaction
        let latest_block_header =
            self.database().latest_header().await?.ok_or(EthApiError::UnknownBlockNumber(None))?;
        validate_transaction(&transaction_signed, chain_id, &latest_block_header)?;

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;

        // Get the number of retries for the transaction
        let retries = self.database().pending_transaction_retries(&transaction_signed.hash).await?;

        // Upsert the transaction as pending in the database
        let transaction =
            from_recovered(TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer));
        self.database().upsert_pending_transaction(transaction, retries).await?;

        // Convert the Ethereum transaction to a Starknet transaction
        let starknet_transaction = to_starknet_transaction(&transaction_signed, signer, retries)?;

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.deploy_evm_transaction_signer(signer).await?;

        // Add the transaction to the Starknet provider
        let span = tracing::span!(tracing::Level::INFO, "sn::add_invoke_transaction");
        let res = self
            .starknet_provider()
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
}
