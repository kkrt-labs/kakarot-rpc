use crate::{
    constants::{ETH_CHAIN_ID, KKRT_BLOCK_GAS_LIMIT},
    pool::{
        mempool::{KakarotPool, TransactionOrdering},
        validate::KakarotTransactionValidatorBuilder,
    },
    providers::{
        eth_provider::{
            database::{types::transaction::ExtendedTransaction, Database},
            error::SignatureError,
            provider::{EthApiResult, EthDataProvider},
            TransactionProvider, TxPoolProvider,
        },
        sn_provider::StarknetProvider,
    },
};
use alloy_eips::eip2718::Encodable2718;
use alloy_primitives::{Address, Bytes, B256};
use alloy_rlp::Decodable;
use alloy_rpc_types_txpool::TxpoolContent;
use async_trait::async_trait;
use reth_chainspec::ChainSpec;
use reth_primitives::{TransactionSigned, TransactionSignedEcRecovered};
use reth_rpc_eth_types::TransactionSource;
use reth_transaction_pool::{
    blobstore::NoopBlobStore, AllPoolTransactions, EthPooledTransaction, PoolConfig, PoolTransaction,
    TransactionOrigin, TransactionPool,
};
use starknet::providers::Provider;
use std::{collections::BTreeMap, sync::Arc};

#[async_trait]
pub trait KakarotTransactions {
    /// Send a raw transaction to the network and returns the transactions hash.
    async fn send_raw_transaction(&self, transaction: Bytes) -> EthApiResult<B256>;
}

#[async_trait]
pub trait TransactionHashProvider {
    /// Returns the transaction by hash.
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<ExtendedTransaction>>;
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
    /// Get the Starknet provider from the Ethereum provider.
    pub const fn starknet_provider(&self) -> &StarknetProvider<SP> {
        self.eth_provider.starknet_provider()
    }

    /// Tries to start a [`EthClient`] by fetching the current chain id, initializing a [`EthDataProvider`] and a [`Pool`].
    pub fn new(starknet_provider: SP, pool_config: PoolConfig, database: Database) -> Self {
        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let eth_provider = EthDataProvider::new(database, StarknetProvider::new(starknet_provider));

        let validator = KakarotTransactionValidatorBuilder::new(&Arc::new(ChainSpec {
            chain: (*ETH_CHAIN_ID).into(),
            max_gas_limit: KKRT_BLOCK_GAS_LIMIT,
            ..Default::default()
        }))
        .build::<_, EthPooledTransaction>(eth_provider.clone());

        let pool = Arc::new(KakarotPool::new(
            validator,
            TransactionOrdering::default(),
            NoopBlobStore::default(),
            pool_config,
        ));

        Self { eth_provider, pool }
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
        let transaction_signed = TransactionSigned::decode(&mut transaction.0.as_ref())?;

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;
        let hash = transaction_signed.hash();
        let to = transaction_signed.to();

        let transaction_signed_ec_recovered =
            TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer);

        let encoded_length = transaction_signed_ec_recovered.clone().encode_2718_len();

        let pool_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.eth_provider.deploy_evm_transaction_signer(signer).await?;

        // Add the transaction to the pool and wait for it to be picked up by a relayer
        let hash = self
            .pool
            .add_transaction(TransactionOrigin::Local, pool_transaction)
            .await
            .inspect_err(|err| tracing::warn!(?err, ?hash, ?to, from = ?signer))?;

        Ok(hash)
    }
}

#[async_trait]
impl<SP> TxPoolProvider for EthClient<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    fn content(&self) -> TxpoolContent<ExtendedTransaction> {
        #[inline]
        fn insert<T: PoolTransaction<Consensus = TransactionSignedEcRecovered>>(
            tx: &T,
            content: &mut BTreeMap<Address, BTreeMap<String, ExtendedTransaction>>,
        ) {
            content.entry(tx.sender()).or_default().insert(
                tx.nonce().to_string(),
                reth_rpc_types_compat::transaction::from_recovered::<reth_rpc::eth::EthTxBuilder>(
                    tx.clone().into_consensus(),
                ),
            );
        }

        let AllPoolTransactions { pending, queued } = self.pool.all_transactions();

        let mut content = TxpoolContent::default();
        for pending in pending {
            insert(&pending.transaction, &mut content.pending);
        }
        for queued in queued {
            insert(&queued.transaction, &mut content.queued);
        }

        content
    }

    async fn txpool_content(&self) -> EthApiResult<TxpoolContent<ExtendedTransaction>> {
        Ok(self.content())
    }
}

#[async_trait]
impl<SP> TransactionHashProvider for EthClient<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn transaction_by_hash(&self, hash: B256) -> EthApiResult<Option<ExtendedTransaction>> {
        Ok(self
            .pool
            .get(&hash)
            .map(|transaction| {
                TransactionSource::Pool(transaction.transaction.transaction().clone())
                    .into_transaction::<reth_rpc::eth::EthTxBuilder>()
            })
            .or(self.eth_provider.transaction_by_hash(hash).await?))
    }
}
