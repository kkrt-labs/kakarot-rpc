use reth_primitives::B256;
use reth_revm::primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, HandlerCfg, SpecId};
use reth_rpc_types::{BlockId, BlockTransactions, Header};

use crate::eth_provider::{
    error::{EthApiError, TransactionError},
    provider::EthereumProvider,
};

use super::{config::KakarotEvmConfig, database::EthDatabaseSnapshot, Tracer, TracerResult};

#[derive(Debug)]
pub struct Floating;
#[derive(Debug)]
pub struct Pinned;

#[derive(Debug)]
pub struct TracerBuilder<P: EthereumProvider + Send + Sync, Status = Floating> {
    eth_provider: P,
    env: Env,
    block: Option<reth_rpc_types::Block>,
    _phantom: std::marker::PhantomData<Status>,
}

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Floating> {
    pub async fn new(eth_provider: P) -> TracerResult<Self> {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = eth_provider.chain_id().await?.unwrap_or_default().to();

        let env = Env { cfg, ..Default::default() };

        Ok(Self { eth_provider, env, block: None, _phantom: std::marker::PhantomData })
    }

    /// Sets the block to trace
    pub async fn with_block_id(self, block_id: BlockId) -> TracerResult<TracerBuilder<P, Pinned>> {
        let maybe_block = self.block(block_id).await?;

        Ok(TracerBuilder {
            eth_provider: self.eth_provider.clone(),
            env: self.env.clone(),
            block: maybe_block,
            _phantom: std::marker::PhantomData,
        })
    }

    /// Fetches a block from the Ethereum provider given a block id
    ///
    /// # Returns
    ///
    /// Returns the block if it exists, otherwise returns None
    async fn block(&self, block_id: BlockId) -> TracerResult<Option<reth_rpc_types::Block>> {
        let maybe_block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.block_hash, true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        };

        if maybe_block.is_none() {
            return Ok(None);
        }

        // we can't trace a pending block
        let block = maybe_block.unwrap();
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock);
        }

        Ok(Some(block.inner))
    }
}

impl<P: EthereumProvider + Send + Sync + Clone> TracerBuilder<P, Pinned> {
    /// Builds the tracer. Returns None if the block was not found during the call to
    /// `with_block_id`.
    pub fn build(self) -> TracerResult<Option<Tracer<P>>> {
        // If block is None, it means that the block was not found
        if self.block.is_none() {
            return Ok(None);
        }
        let block = self.block.as_ref().unwrap();

        let transactions = match &block.transactions {
            BlockTransactions::Full(transactions) => transactions.clone(),
            _ => return Err(TransactionError::ExpectedFullTransactions.into()),
        };

        let env = self.init_env_with_handler_config();
        // DB should use the state of the parent block
        let db = EthDatabaseSnapshot::new(self.eth_provider, BlockId::Hash(block.header.parent_hash.into()));

        Ok(Some(Tracer { env, transactions, cfg: KakarotEvmConfig, db }))
    }

    /// Init an EnvWithHandlerCfg.
    fn init_env_with_handler_config(&self) -> EnvWithHandlerCfg {
        let env = Box::new(self.init_env_with_block_env());
        EnvWithHandlerCfg::new(env, HandlerCfg::new(SpecId::CANCUN))
    }

    /// Inits the Env by using `self.block` to set the block environment.
    fn init_env_with_block_env(&self) -> Env {
        let mut env = self.env.clone();

        let Header { number, timestamp, gas_limit, miner, base_fee_per_gas, difficulty, .. } =
            self.block.clone().expect("Block not set").header;
        let block_env = BlockEnv {
            number: number.unwrap_or_default(),
            timestamp,
            gas_limit,
            coinbase: miner,
            basefee: base_fee_per_gas.unwrap_or_default(),
            prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes::<32>()[..])),
            ..Default::default()
        };
        env.block = block_env;
        env
    }
}
