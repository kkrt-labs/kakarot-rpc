mod database;

use crate::eth_provider::{
    error::EthApiError,
    provider::{EthDataProvider, EthereumProvider},
};
use reth_primitives::{
    revm,
    revm_primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, SpecId},
    B256, U256,
};
use reth_rpc_types::{trace::parity::LocalizedTransactionTrace, BlockId, Header};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

pub type TracerResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
pub struct Tracer<'a, SP: starknet::providers::Provider> {
    eth_provider: &'a EthDataProvider<SP>,
    env: Env,
    inspector: TracingInspector,
}

impl<'a, SP> Tracer<'a, SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    pub async fn new(eth_provider: &'a EthDataProvider<SP>) -> TracerResult<Self> {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = eth_provider.chain_id().await?.unwrap_or_default().to();

        let env = Env { cfg, ..Default::default() };

        Ok(Self { eth_provider, env, inspector: TracingInspector::new(TracingInspectorConfig::default_parity()) })
    }

    pub async fn trace_block(&mut self, block_id: BlockId) -> TracerResult<Vec<LocalizedTransactionTrace>> {
        let block = match block_id {
            BlockId::Hash(hash) => {
                self.eth_provider.block_by_hash(hash.block_hash, true).await?.ok_or(EthApiError::UnknownBlock)?
            }
            BlockId::Number(number) => {
                self.eth_provider.block_by_number(number, true).await?.ok_or(EthApiError::UnknownBlock)?
            }
        };
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock.into());
        }

        let Header { number, timestamp, gas_limit, miner, base_fee_per_gas, difficulty, .. } = block.header;

        let block_env = BlockEnv {
            number: number.unwrap_or_default().to(),
            timestamp,
            gas_limit,
            coinbase: miner,
            basefee: base_fee_per_gas.unwrap_or_default(),
            difficulty: U256::ZERO,
            prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes()[..])),
            ..Default::default()
        };
        self.env.block = block_env;

        let env = EnvWithHandlerCfg::new(self.env.clone().into(), SpecId::CANCUN);

        let ctx = revm_core::Context::new(revm_core::EvmContext::new_with_env(db, env), revm::State::default());

        // let mut traces = Vec::with_capacity(block.transactions.len());

        Err(eyre::eyre!("Not implemented yet!").into())
    }
}
