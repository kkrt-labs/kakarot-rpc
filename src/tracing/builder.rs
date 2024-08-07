use super::{Tracer, TracerResult};
use crate::providers::eth_provider::{
    database::state::{EthCacheDatabase, EthDatabase},
    error::{EthApiError, TransactionError},
    provider::EthereumProvider,
};
use reth_primitives::{B256, U256};
use reth_revm::{
    db::CacheDB,
    primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, HandlerCfg, SpecId},
};
use reth_rpc_types::{
    trace::geth::{GethDebugTracingCallOptions, GethDebugTracingOptions},
    Block, BlockHashOrNumber, BlockId, BlockTransactions, Header,
};
use revm_inspectors::tracing::TracingInspectorConfig;

#[derive(Debug, Clone)]
pub struct Floating;
#[derive(Debug)]
pub struct Pinned;

/// Representing different tracing options for transactions.
#[derive(Clone, Debug)]
pub enum TracingOptions {
    /// Geth debug tracing options.
    Geth(GethDebugTracingOptions),
    /// Parity tracing options.
    Parity(TracingInspectorConfig),
    /// Geth debug call tracing options.
    GethCall(GethDebugTracingCallOptions),
}

impl TracingOptions {
    /// Returns `Some` with a reference to [`GethDebugTracingOptions`] if this is `Geth`,
    /// otherwise returns `None`.
    pub const fn as_geth(&self) -> Option<&GethDebugTracingOptions> {
        if let Self::Geth(ref options) = self {
            Some(options)
        } else {
            None
        }
    }

    /// Returns `Some` with a reference to [`TracingInspectorConfig`] if this is `Parity`,
    /// otherwise returns `None`.
    pub const fn as_parity(&self) -> Option<&TracingInspectorConfig> {
        if let Self::Parity(ref config) = self {
            Some(config)
        } else {
            None
        }
    }

    /// Returns `Some` with a reference to [`GethDebugTracingCallOptions`] if this is `GethCall`,
    /// otherwise returns `None`.
    pub const fn as_geth_call(&self) -> Option<&GethDebugTracingCallOptions> {
        if let Self::GethCall(ref options) = self {
            Some(options)
        } else {
            None
        }
    }
}

impl Default for TracingOptions {
    fn default() -> Self {
        GethDebugTracingOptions::default().into()
    }
}

impl From<GethDebugTracingOptions> for TracingOptions {
    fn from(options: GethDebugTracingOptions) -> Self {
        Self::Geth(options)
    }
}

impl From<TracingInspectorConfig> for TracingOptions {
    fn from(config: TracingInspectorConfig) -> Self {
        Self::Parity(config)
    }
}

impl From<GethDebugTracingCallOptions> for TracingOptions {
    fn from(options: GethDebugTracingCallOptions) -> Self {
        Self::GethCall(options)
    }
}

#[derive(Debug, Clone)]
pub struct TracerBuilder<P: EthereumProvider + Send + Sync + Clone, Status = Floating> {
    eth_provider: P,
    env: Env,
    block: Block,
    tracing_options: TracingOptions,
    _phantom: std::marker::PhantomData<Status>,
}

/// Block gas limit for tracing. Set to an arbitrarily high value to never run out.
/// Block gas limit is only partially enforced in Cairo EVM layer: <https://github.com/kkrt-labs/kakarot/blob/98b26fda32c36f09880ed0c7f44dba7f4d669b61/src/kakarot/accounts/library.cairo#L245>
/// Remove when block gas limit is enforced consistently (i.e. when we check that a transaction's gas limit is lower than the block gas limit as well as the current block's cumulative gas)
pub const TRACING_BLOCK_GAS_LIMIT: u64 = 1_000_000_000;

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Floating> {
    pub async fn new(eth_provider: P) -> TracerResult<Self> {
        let cfg = CfgEnv::default().with_chain_id(eth_provider.chain_id().await?.unwrap_or_default().to());

        let env = Env { cfg, ..Default::default() };

        Ok(Self {
            eth_provider,
            env,
            block: Default::default(),
            tracing_options: Default::default(),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Sets the block to trace
    pub async fn with_block_id(self, block_id: BlockId) -> TracerResult<TracerBuilder<P, Pinned>> {
        let block = self.block(block_id).await?;

        Ok(TracerBuilder {
            eth_provider: self.eth_provider.clone(),
            env: self.env.clone(),
            block,
            tracing_options: self.tracing_options.clone(),
            _phantom: std::marker::PhantomData,
        })
    }

    /// Sets the block to trace given the transaction hash
    pub async fn with_transaction_hash(self, transaction_hash: B256) -> TracerResult<TracerBuilder<P, Pinned>> {
        let transaction = self
            .eth_provider
            .transaction_by_hash(transaction_hash)
            .await?
            .ok_or(EthApiError::TransactionNotFound(transaction_hash))?;

        // we can't trace a pending transaction
        if transaction.block_number.is_none() {
            return Err(EthApiError::TransactionNotFound(transaction_hash));
        }

        self.with_block_id(BlockId::Number(transaction.block_number.unwrap().into())).await
    }

    /// Fetches a block from the Ethereum provider given a block id
    ///
    /// # Returns
    ///
    /// Returns the block if it exists, otherwise returns None
    async fn block(&self, block_id: BlockId) -> TracerResult<reth_rpc_types::Block> {
        let block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.block_hash, true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        }
        .ok_or(match block_id {
            BlockId::Hash(hash) => EthApiError::UnknownBlock(hash.block_hash.into()),
            BlockId::Number(number) => {
                EthApiError::UnknownBlock(BlockHashOrNumber::Number(number.as_number().unwrap_or_default()))
            }
        })?;

        // we can't trace a pending block
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock(BlockHashOrNumber::Hash(B256::ZERO)));
        }

        Ok(block.inner)
    }
}

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Pinned> {
    /// Sets the tracing options
    #[must_use]
    pub fn with_tracing_options(mut self, tracing_options: TracingOptions) -> Self {
        self.tracing_options = tracing_options;
        self
    }

    /// Builds the tracer.
    pub fn build(self) -> TracerResult<Tracer<P>> {
        let transactions = match &self.block.transactions {
            BlockTransactions::Full(transactions) => transactions.clone(),
            _ => return Err(TransactionError::ExpectedFullTransactions.into()),
        };

        let env = self.init_env_with_handler_config();
        // DB should use the state of the parent block
        let db =
            EthCacheDatabase(CacheDB::new(EthDatabase::new(self.eth_provider, self.block.header.parent_hash.into())));

        let tracing_options = self.tracing_options;

        Ok(Tracer { transactions, env, db, tracing_options })
    }

    /// Init an `EnvWithHandlerCfg`.
    fn init_env_with_handler_config(&self) -> EnvWithHandlerCfg {
        let env = Box::new(self.init_env_with_block_env());
        EnvWithHandlerCfg::new(env, HandlerCfg::new(SpecId::CANCUN))
    }

    /// Inits the Env by using `self.block` to set the block environment.
    fn init_env_with_block_env(&self) -> Env {
        let mut env = self.env.clone();

        let Header { number, timestamp, miner, base_fee_per_gas, difficulty, .. } = self.block.header.clone();
        let block_env = BlockEnv {
            number: U256::from(number.unwrap_or_default()),
            timestamp: U256::from(timestamp),
            gas_limit: U256::from(TRACING_BLOCK_GAS_LIMIT),
            coinbase: miner,
            basefee: U256::from(base_fee_per_gas.unwrap_or_default()),
            prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes::<32>()[..])),
            ..Default::default()
        };
        env.block = block_env;
        env
    }
}

// The following tests validates the behavior of the TracerBuilder when interacting with a mock Ethereum provider.
// Each test focuses on different scenarios where the TracerBuilder is expected to handle various errors correctly,
// such as unknown blocks, not found transactions, and invalid chain IDs.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::mock_provider::MockEthereumProviderStruct;
    use reth_primitives::U64;
    use reth_rpc_types::Transaction;
    use std::sync::Arc;
    #[tokio::test]
    async fn test_tracer_builder_block_failure_with_none_block_number() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return 1
        mock_provider.expect_chain_id().returning(|| Ok(Some(U64::from(1))));
        // Expect the block_by_number call to return an error for an unknown block
        mock_provider.expect_block_by_number().returning(|_, _| Ok(None));

        // Create a TracerBuilder with the mock provider
        let builder = TracerBuilder::new(Arc::new(&mock_provider)).await.unwrap();
        // Attempt to use the builder with a specific block ID, expecting an error
        let result = builder.block(BlockId::Number(1.into())).await;
        // Check that the result is an UnknownBlock error
        assert!(matches!(result, Err(EthApiError::UnknownBlock(_))));
    }

    #[tokio::test]
    async fn test_tracer_builder_block_failure_with_none_block_hash() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return 1
        mock_provider.expect_chain_id().returning(|| Ok(Some(U64::from(1))));
        // Expect the block_by_hash call to return an error for an unknown block
        mock_provider.expect_block_by_hash().returning(|_, _| Ok(None));

        // Create a TracerBuilder with the mock provider
        let builder = TracerBuilder::new(Arc::new(&mock_provider)).await.unwrap();
        // Attempt to use the builder with a specific block hash, expecting an error
        let result = builder.block(BlockId::Hash(B256::repeat_byte(1).into())).await;
        // Check that the result is an UnknownBlock error
        assert!(matches!(result, Err(EthApiError::UnknownBlock(_))));
    }

    #[tokio::test]
    async fn test_tracer_builder_with_transaction_not_found() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return 1
        mock_provider.expect_chain_id().returning(|| Ok(Some(U64::from(1))));
        // Expect the transaction_by_hash call to return Ok(None) for not found transaction
        mock_provider.expect_transaction_by_hash().returning(|_| Ok(None));

        // Create a TracerBuilder with the mock provider
        let builder = TracerBuilder::new(Arc::new(&mock_provider)).await.unwrap();
        // Attempt to use the builder with a specific transaction hash, expecting an error
        let result = builder.with_transaction_hash(B256::repeat_byte(0)).await;
        // Check that the result is a TransactionNotFound error
        assert!(matches!(result, Err(EthApiError::TransactionNotFound(_))));
    }

    #[tokio::test]
    async fn test_tracer_builder_with_unknown_block() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return 1
        mock_provider.expect_chain_id().returning(|| Ok(Some(U64::from(1))));
        // Expect the transaction_by_hash call to return a transaction with no block number
        mock_provider
            .expect_transaction_by_hash()
            .returning(|_| Ok(Some(Transaction { block_number: None, ..Default::default() })));

        // Create a TracerBuilder with the mock provider
        let builder = TracerBuilder::new(Arc::new(&mock_provider)).await.unwrap();
        // Attempt to use the builder with a specific transaction hash, expecting an error
        let result = builder.with_transaction_hash(B256::repeat_byte(0)).await;
        // Check that the result is an UnknownBlock error
        assert!(matches!(result, Err(EthApiError::TransactionNotFound(_))));
    }

    #[tokio::test]
    async fn test_tracer_builder_failure() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return an error
        mock_provider.expect_chain_id().returning(|| Err(TransactionError::InvalidChainId.into()));
        // Attempt to create a TracerBuilder with the mock provider, expecting an error
        let result = TracerBuilder::new(Arc::new(&mock_provider)).await;
        // Check that the result is an error
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_tracer_builder_build_error() {
        // Create a mock Ethereum provider
        let mut mock_provider = MockEthereumProviderStruct::new();
        // Expect the chain_id call to return 1
        mock_provider.expect_chain_id().returning(|| Ok(Some(U64::from(1))));
        // Expect the block_by_number call to return a block with non-full transactions
        mock_provider.expect_block_by_number().returning(|_, _| {
            Ok(Some(
                Block {
                    transactions: BlockTransactions::Hashes(vec![]),
                    header: Header { hash: Some(B256::repeat_byte(1)), ..Default::default() },
                    ..Default::default()
                }
                .into(),
            ))
        });

        // Create a TracerBuilder with the mock provider
        let builder = TracerBuilder::new(Arc::new(&mock_provider)).await.unwrap();
        // Attempt to use the builder with a specific block ID
        let builder = builder.with_block_id(BlockId::Number(1.into())).await.unwrap();
        // Attempt to build the tracer, expecting an error
        let result = builder.build();
        // Check that the result is an ExpectedFullTransactions error
        assert!(matches!(result, Err(EthApiError::Transaction(TransactionError::ExpectedFullTransactions))));
    }
}
