mod database;

use crate::eth_provider::{
    error::{EthApiError, TransactionError},
    provider::EthereumProvider,
};
use reth_primitives::{
    revm_primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, SpecId},
    B256, U256,
};
use reth_revm::tracing::{TracingInspector, TracingInspectorConfig};
use reth_revm::{
    primitives::{HandlerCfg, TransactTo},
    DatabaseCommit,
};
use reth_rpc_types::{trace::parity::LocalizedTransactionTrace, BlockId, BlockTransactions, Header, TransactionInfo};

pub type TracerResult<T> = Result<T, EthApiError>;

#[derive(Debug)]
pub struct Tracer<P: EthereumProvider + Send + Sync> {
    eth_provider: P,
    env: Env,
    inspector: TracingInspector,
}

impl<P: EthereumProvider + Send + Sync + Clone> Tracer<P> {
    pub async fn new(eth_provider: P) -> TracerResult<Self> {
        let mut cfg = CfgEnv::default();
        cfg.chain_id = eth_provider.chain_id().await?.unwrap_or_default().to();

        let env = Env { cfg, ..Default::default() };

        Ok(Self { eth_provider, env, inspector: TracingInspector::new(TracingInspectorConfig::default_parity()) })
    }

    pub async fn trace_block(&self, block_id: BlockId) -> TracerResult<Option<Vec<LocalizedTransactionTrace>>> {
        let maybe_block = match block_id {
            BlockId::Hash(hash) => self.eth_provider.block_by_hash(hash.block_hash, true).await?,
            BlockId::Number(number) => self.eth_provider.block_by_number(number, true).await?,
        };

        if maybe_block.is_none() {
            return Ok(None);
        }

        dbg!(&maybe_block);

        let block = maybe_block.unwrap();
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock);
        }

        let Header { number, timestamp, gas_limit, miner, base_fee_per_gas, difficulty, .. } = block.header;

        let block_env = BlockEnv {
            number: number.unwrap_or_default().to(),
            timestamp,
            gas_limit,
            coinbase: miner,
            basefee: base_fee_per_gas.unwrap_or_default(),
            difficulty: U256::ZERO,
            prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes::<32>()[..])),
            ..Default::default()
        };
        let mut env = self.env.clone();
        env.block = block_env;

        let db = database::EthDatabaseSnapshot::new(self.eth_provider.clone(), block_id);
        let ctx = reth_revm::Context::new(reth_revm::EvmContext::new_with_env(db, env.into()), self.inspector.clone());

        let env = EnvWithHandlerCfg::new(self.env.clone().into(), HandlerCfg::new(SpecId::CANCUN));
        let mut handler = reth_revm::Handler::new(env.handler_cfg);
        handler.append_handler_register_plain(reth_revm::inspector_handle_register);
        let mut evm = reth_revm::Evm::new(ctx, handler);

        dbg!("entering tracing");

        let traces = tokio::task::block_in_place(move || {
            let transactions = match &block.transactions {
                BlockTransactions::Full(transactions) => transactions,
                _ => return Err(TransactionError::ExpectedFullTransactions.into()),
            };
            let mut traces = Vec::with_capacity(block.transactions.len());

            for tx in transactions {
                dbg!(tx);
                evm = evm
                    .modify()
                    .modify_tx_env(|tx_env| {
                        tx_env.caller = tx.from;
                        tx_env.gas_limit = tx.gas.to();
                        tx_env.gas_price = tx.gas_price.unwrap_or_default();
                        tx_env.transact_to = tx.to.map(TransactTo::call).unwrap_or_else(TransactTo::create);
                        tx_env.value = tx.value;
                        tx_env.data = tx.input.clone();
                        tx_env.nonce = Some(tx.nonce);
                        tx_env.chain_id = tx.chain_id;
                        tx_env.access_list = tx
                            .access_list
                            .clone()
                            .map(|al| {
                                al.0.into_iter()
                                    .map(|item| {
                                        (
                                            item.address,
                                            item.storage_keys
                                                .into_iter()
                                                .map(|slot| U256::from_be_bytes(slot.0))
                                                .collect(),
                                        )
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();
                        tx_env.gas_priority_fee = tx.max_priority_fee_per_gas;
                    })
                    .build();
                let result = evm.transact().map_err(|err| TransactionError::TracingError(err.into()))?;
                evm.context.evm.inner.db.commit(result.state);

                let parity_builder = evm.context.external.clone().into_parity_builder();
                let transaction_info = TransactionInfo {
                    hash: Some(tx.hash),
                    index: tx.transaction_index.map(|i| i.to()),
                    block_hash: tx.block_hash,
                    block_number: tx.block_number.map(|i| i.to()),
                    base_fee: tx
                        .max_fee_per_gas
                        .map(|fee| (fee - tx.max_priority_fee_per_gas.unwrap_or_default()).to()),
                };
                traces.extend(parity_builder.into_localized_transaction_traces(transaction_info));
            }

            Result::<_, EthApiError>::Ok(traces)
        })?;

        Ok(Some(traces))
    }
}
