use crate::{
    pool::{
        mempool::{KakarotPool, TransactionOrdering},
        validate::KakarotTransactionValidatorBuilder,
    },
    providers::{
        eth_provider::{
            database::Database,
            error::{EthApiError, EthereumDataFormatError, KakarotError, SignatureError},
            provider::{EthApiResult, EthDataProvider},
            TxPoolProvider,
        },
        sn_provider::StarknetProvider,
    },
};
use alloy_rlp::Decodable;
use async_trait::async_trait;
use num_traits::ToPrimitive;
use reth_chainspec::ChainSpec;
use reth_primitives::{Address, Bytes, TransactionSigned, TransactionSignedEcRecovered, B256};
use reth_rpc_types::{txpool::TxpoolContent, Transaction, WithOtherFields};
use reth_transaction_pool::{
    blobstore::NoopBlobStore, AllPoolTransactions, EthPooledTransaction, PoolConfig, PoolTransaction,
    TransactionOrigin, TransactionPool,
};
use starknet::{core::types::Felt, providers::Provider};
use std::{collections::BTreeMap, sync::Arc};

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
    /// Get the Starknet provider from the Ethereum provider.
    pub const fn starknet_provider(&self) -> &StarknetProvider<SP> {
        self.eth_provider.starknet_provider()
    }

    /// Tries to start a [`EthClient`] by fetching the current chain id, initializing a [`EthDataProvider`] and a [`Pool`].
    pub async fn try_new(starknet_provider: SP, database: Database) -> eyre::Result<Self> {
        let chain = (starknet_provider.chain_id().await.map_err(KakarotError::from)?.to_bigint()
            & Felt::from(u32::MAX).to_bigint())
        .to_u64()
        .unwrap();

        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let eth_provider = EthDataProvider::try_new(database, StarknetProvider::new(starknet_provider)).await?;

        let validator =
            KakarotTransactionValidatorBuilder::new(&Arc::new(ChainSpec { chain: chain.into(), ..Default::default() }))
                .build::<_, EthPooledTransaction>(eth_provider.clone());

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

        // Recover the signer from the transaction
        let signer = transaction_signed.recover_signer().ok_or(SignatureError::Recovery)?;
        let transaction_signed_ec_recovered =
            TransactionSignedEcRecovered::from_signed_transaction(transaction_signed.clone(), signer);

        let encoded_length = transaction_signed_ec_recovered.clone().length_without_header();
        let pool_transaction = EthPooledTransaction::new(transaction_signed_ec_recovered, encoded_length);

        // Deploy EVM transaction signer if Hive feature is enabled
        #[cfg(feature = "hive")]
        self.eth_provider.deploy_evm_transaction_signer(signer).await?;

        // Add the transaction to the pool and wait for it to be picked up by a relayer
        let hash = self.pool.add_transaction(TransactionOrigin::Local, pool_transaction).await?;

        Ok(hash)
    }
}

#[async_trait]
impl<SP> TxPoolProvider for EthClient<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    fn content(&self) -> TxpoolContent<WithOtherFields<Transaction>> {
        #[inline]
        fn insert<T: PoolTransaction<Consensus = TransactionSignedEcRecovered>>(
            tx: &T,
            content: &mut BTreeMap<Address, BTreeMap<String, WithOtherFields<Transaction>>>,
        ) {
            content.entry(tx.sender()).or_default().insert(
                tx.nonce().to_string(),
                reth_rpc_types_compat::transaction::from_recovered(tx.clone().into_consensus()),
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

    async fn txpool_content(&self) -> EthApiResult<TxpoolContent<WithOtherFields<Transaction>>> {
        Ok(self.content())
    }
}
