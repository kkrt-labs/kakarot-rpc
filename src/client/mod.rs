use crate::{
    models::transaction::validate_transaction,
    pool::{
        mempool::{KakarotPool, TransactionOrdering},
        validate::KakarotTransactionValidatorBuilder,
    },
    providers::eth_provider::{
        chain::ChainProvider,
        database::{
            ethereum::{EthereumBlockStore, EthereumTransactionStore},
            state::EthDatabase,
            Database,
        },
        error::{EthApiError, EthereumDataFormatError, KakarotError, SignatureError, TransactionError},
        provider::{EthApiResult, EthDataProvider},
        starknet::kakarot_core::to_starknet_transaction,
    },
};
use alloy_rlp::Decodable;
use async_trait::async_trait;
use num_traits::ToPrimitive;
use reth_chainspec::ChainSpec;
use reth_primitives::{Bytes, TransactionSigned, TransactionSignedEcRecovered, B256};
use reth_rpc_types_compat::transaction::from_recovered;
use reth_transaction_pool::{
    blobstore::NoopBlobStore, EthPooledTransaction, PoolConfig, TransactionOrigin, TransactionPool,
};
use starknet::{core::types::Felt, providers::Provider};
use std::sync::Arc;
use tracing::Instrument;

#[async_trait]
pub trait KakarotTransactions {
    /// Send a raw transaction to the network and returns the transactions hash.
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthApiResult<B256>;
}

/// Provides a wrapper structure around the Ethereum Provider
/// and the Mempool.
#[derive(Debug, Clone)]
pub struct EthClient<SP: Provider + Send + Sync> {
    eth_provider: EthDataProvider<SP>,
    pool: Arc<KakarotPool<EthDataProvider<SP>>>,
}

impl<SP> EthClient<SP>
where
    SP: Provider + Clone + Sync + Send,
{
    /// Tries to start a [`EthClient`] by fetching the current chain id, initializing a [`EthDataProvider`] and a `Pool`.
    pub async fn try_new(starknet_provider: SP, database: Database) -> eyre::Result<Self> {
        let chain = (starknet_provider.chain_id().await.map_err(KakarotError::from)?.to_bigint()
            & Felt::from(u32::MAX).to_bigint())
        .to_u64()
        .unwrap();

        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let eth_provider = EthDataProvider::try_new(database, starknet_provider).await?;

        let validator =
            KakarotTransactionValidatorBuilder::new(Arc::new(ChainSpec { chain: chain.into(), ..Default::default() }))
                .build::<_, EthPooledTransaction>(EthDatabase::new(eth_provider.clone(), 0.into()));

        let pool = Arc::new(KakarotPool::new(
            validator,
            TransactionOrdering::default(),
            NoopBlobStore::default(),
            PoolConfig::default(),
        ));

        Ok(Self { eth_provider, pool })
    }

    /// Returns a clone of the [`EthDataProvider`]
    pub const fn eth_provider(&self) -> &EthDataProvider<SP> {
        &self.eth_provider
    }

    /// Returns a clone of the `Pool`
    pub fn mempool(&self) -> Arc<KakarotPool<EthDataProvider<SP>>> {
        self.pool.clone()
    }
}

#[async_trait]
impl<SP> KakarotTransactions for EthClient<SP>
where
    SP: Provider + Clone + Sync + Send,
{
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthApiResult<B256> {
        // Decode the transaction data
        let transaction_signed = TransactionSigned::decode(&mut transaction.0.as_ref())
            .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?;

        let chain_id: u64 = self
            .eth_provider
            .chain_id()
            .await?
            .unwrap_or_default()
            .try_into()
            .map_err(|_| TransactionError::InvalidChainId)?;

        // Validate the transaction
        let latest_block_header =
            self.eth_provider.database().latest_header().await?.ok_or(EthApiError::UnknownBlockNumber(None))?;
        validate_transaction(&transaction_signed, chain_id, &latest_block_header)?;

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;

        // Get the number of retries for the transaction
        let retries = self.eth_provider.database().pending_transaction_retries(&transaction_signed.hash).await?;

        let transaction_signed_ec_recovered =
            TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer);

        let encoded_length = transaction_signed_ec_recovered.clone().length_without_header();

        // Upsert the transaction as pending in the database
        let transaction = from_recovered(transaction_signed_ec_recovered.clone());
        self.eth_provider.database().upsert_pending_transaction(transaction, retries).await?;

        // Convert the Ethereum transaction to a Starknet transaction
        let starknet_transaction = to_starknet_transaction(&transaction_signed, signer, retries)?;

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.eth_provider.deploy_evm_transaction_signer(signer).await?;

        // Add the transaction to the Starknet provider
        let span = tracing::span!(tracing::Level::INFO, "sn::add_invoke_transaction");
        let res = self
            .eth_provider
            .starknet_provider()
            .add_invoke_transaction(starknet_transaction)
            .instrument(span)
            .await
            .map_err(KakarotError::from)?;

        let pool_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);

        // Don't handle the result in case we are adding multiple times the same transaction due to the retry.
        let _ = self.pool.as_ref().add_transaction(TransactionOrigin::Local, pool_transaction).await;

        // Return transaction hash if testing feature is enabled, otherwise log and return Ethereum hash
        if cfg!(feature = "testing") {
            return Ok(B256::from_slice(&res.transaction_hash.to_bytes_be()[..]));
        }
        let hash = transaction_signed.hash();
        tracing::info!(
            ethereum_hash = ?hash,
            starknet_hash = ?B256::from_slice(&res.transaction_hash.to_bytes_be()[..]),
        );

        Ok(hash)
    }
}
