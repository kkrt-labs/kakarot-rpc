use reth_primitives::{B256, U256};
use reth_revm::primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, HandlerCfg, SpecId};
use reth_rpc_types::{Block, BlockId, BlockTransactions, Header};

use crate::eth_provider::{
    error::{EthApiError, TransactionError},
    provider::EthereumProvider,
};

use super::{database::EthDatabaseSnapshot, Tracer, TracerResult};

#[derive(Debug)]
pub struct Floating;
#[derive(Debug)]
pub struct Pinned;

#[derive(Debug)]
pub struct TracerBuilder<P: EthereumProvider + Send + Sync, Status = Floating> {
    eth_provider: P,
    env: Env,
    block: Block,
    _phantom: std::marker::PhantomData<Status>,
}

/// Block gas limit for tracing. Set to an arbitrarily high value to never run out.
/// Block gas limit is only partially enforced in Cairo EVM layer: <https://github.com/kkrt-labs/kakarot/blob/98b26fda32c36f09880ed0c7f44dba7f4d669b61/src/kakarot/accounts/library.cairo#L245>
/// Remove when block gas limit is enforced consistently (i.e. when we check that a transaction's gas limit is lower than the block gas limit as well as the current block's cumulative gas)
pub const TRACING_BLOCK_GAS_LIMIT: u64 = 1_000_000_000;

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Floating> {
    pub async fn new(eth_provider: P) -> TracerResult<Self> {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = eth_provider.chain_id().await?.unwrap_or_default().to();

        let env = Env { cfg, ..Default::default() };

        Ok(Self { eth_provider, env, block: Default::default(), _phantom: std::marker::PhantomData })
    }

    /// Sets the block to trace
    pub async fn with_block_id(self, block_id: BlockId) -> TracerResult<TracerBuilder<P, Pinned>> {
        let block = self.block(block_id).await?;

        Ok(TracerBuilder {
            eth_provider: self.eth_provider.clone(),
            env: self.env.clone(),
            block,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Sets the block to trace given the transaction hash
    pub async fn with_transaction_hash(self, transaction_hash: B256) -> TracerResult<TracerBuilder<P, Pinned>> {
        let transaction =
            self.eth_provider.transaction_by_hash(transaction_hash).await?.ok_or(EthApiError::TransactionNotFound)?;

        // we can't trace a pending transaction
        if transaction.block_number.is_none() {
            return Err(EthApiError::UnknownBlock);
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
        .ok_or(EthApiError::UnknownBlock)?;

        // we can't trace a pending block
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock);
        }

        Ok(block.inner)
    }
}

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Pinned> {
    /// Builds the tracer.
    pub fn build(self) -> TracerResult<Tracer<P>> {
        let transactions = match &self.block.transactions {
            BlockTransactions::Full(transactions) => transactions.clone(),
            _ => return Err(TransactionError::ExpectedFullTransactions.into()),
        };

        let env = self.init_env_with_handler_config();
        // DB should use the state of the parent block
        let db = EthDatabaseSnapshot::new(self.eth_provider, BlockId::Hash(self.block.header.parent_hash.into()));

        Ok(Tracer { transactions, env, db })
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
