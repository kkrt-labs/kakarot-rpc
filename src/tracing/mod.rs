pub mod builder;
mod config;
mod database;

use eyre::eyre;
use reth_primitives::revm::env::tx_env_with_recovered;
use reth_primitives::ruint::FromUintError;
use reth_revm::primitives::{Env, EnvWithHandlerCfg};
use reth_revm::DatabaseCommit;
use reth_rpc_types::trace::geth::TraceResult;
use reth_rpc_types::{
    trace::{
        geth::{GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions},
        parity::LocalizedTransactionTrace,
    },
    TransactionInfo,
};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

use self::config::KakarotEvmConfig;
use self::database::EthDatabaseSnapshot;
use crate::{
    eth_provider::{
        error::{EthApiError, TransactionError},
        provider::EthereumProvider,
    },
    models::transaction::rpc_to_ec_recovered_transaction,
};

pub type TracerResult<T> = Result<T, EthApiError>;

#[derive(Debug)]
pub struct Tracer<P: EthereumProvider + Send + Sync> {
    transactions: Vec<reth_rpc_types::Transaction>,
    cfg: KakarotEvmConfig,
    env: EnvWithHandlerCfg,
    db: EthDatabaseSnapshot<P>,
}

impl<P: EthereumProvider + Send + Sync + Clone> Tracer<P> {
    /// Trace the block in the parity format.
    pub fn trace_block(
        self,
        tracing_config: TracingInspectorConfig,
    ) -> TracerResult<Option<Vec<LocalizedTransactionTrace>>> {
        let transact_to_parity_trace =
            |cfg: KakarotEvmConfig,
             env: EnvWithHandlerCfg,
             db: &mut EthDatabaseSnapshot<P>,
             tx: &reth_rpc_types::Transaction|
             -> TracerResult<(Vec<LocalizedTransactionTrace>, reth_revm::primitives::State)> {
                let block_base_fee = env
                    .env
                    .block
                    .basefee
                    .try_into()
                    .map_err(|err: FromUintError<u128>| TransactionError::Tracing(err.into()))?;

                // Set up the inspector and transact the transaction
                let mut inspector = TracingInspector::new(tracing_config);
                let mut evm = cfg.evm_with_env_and_inspector(db, env, &mut inspector);
                let res = evm.transact().map_err(|err| TransactionError::Tracing(err.into()))?;
                // we drop the evm to avoid cloning the inspector
                drop(evm);

                let parity_builder = inspector.into_parity_builder();

                let transaction_info = TransactionInfo {
                    hash: Some(tx.hash),
                    index: tx.transaction_index,
                    block_hash: tx.block_hash,
                    block_number: tx.block_number,
                    base_fee: Some(block_base_fee),
                };

                Ok((parity_builder.into_localized_transaction_traces(transaction_info), res.state))
            };

        let traces = self.trace_block_in_place(transact_to_parity_trace)?;

        Ok(Some(traces))
    }

    /// Returns the debug trace in the Geth.
    /// Currently only supports the call tracer or the default tracer.
    pub fn debug_block(self, opts: GethDebugTracingOptions) -> TracerResult<Option<Vec<TraceResult>>> {
        let transact_to_geth_trace = |cfg: KakarotEvmConfig,
                                      env: EnvWithHandlerCfg,
                                      db: &mut EthDatabaseSnapshot<P>,
                                      tx: &reth_rpc_types::Transaction|
         -> TracerResult<(Vec<TraceResult>, reth_revm::primitives::State)> {
            let GethDebugTracingOptions { tracer_config, config, tracer, .. } = opts.clone();

            if let Some(tracer) = tracer {
                return match tracer {
                    GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer) => {
                        let call_config = tracer_config
                            .clone()
                            .into_call_config()
                            .map_err(|err| EthApiError::Transaction(TransactionError::Tracing(err.into())))?;
                        let mut inspector =
                            TracingInspector::new(TracingInspectorConfig::from_geth_call_config(&call_config));
                        let mut evm = cfg.evm_with_env_and_inspector(db, env, &mut inspector);

                        let res = evm.transact().map_err(|err| TransactionError::Tracing(err.into()))?;
                        // we drop the evm to avoid cloning the inspector
                        drop(evm);
                        let call_frame = inspector.into_geth_builder().geth_call_traces(
                            tracer_config.into_call_config().map_err(|err| TransactionError::Tracing(err.into()))?,
                            res.result.gas_used(),
                        );
                        Ok((
                            vec![TraceResult::Success { result: call_frame.into(), tx_hash: Some(tx.hash) }],
                            res.state,
                        ))
                    }
                    _ => Err(EthApiError::Transaction(TransactionError::Tracing(
                        eyre!("only call tracer is currently supported").into(),
                    ))),
                };
            }

            // default tracer
            let mut inspector = TracingInspector::new(TracingInspectorConfig::from_geth_config(&config));
            let mut evm = cfg.evm_with_env_and_inspector(db, env, &mut inspector);

            let res = evm.transact().map_err(|err| TransactionError::Tracing(err.into()))?;
            // we drop the evm to avoid cloning the inspector
            drop(evm);
            let gas_used = res.result.gas_used();
            let return_value = res.result.into_output().unwrap_or_default();
            let frame = inspector.into_geth_builder().geth_traces(gas_used, return_value, config);
            Ok((vec![TraceResult::Success { result: frame.into(), tx_hash: Some(tx.hash) }], res.state))
        };

        let traces = self.trace_block_in_place(transact_to_geth_trace)?;

        Ok(Some(traces))
    }

    /// Traces a block using tokio::task::block_in_place. This is needed in order to enter a blocking context
    /// which is then converted to a async context in the implementation of [Database] using
    /// `Handle::current().block_on(async { ... })`
    /// The function `transact_and_get_traces` closure uses the `cfg`, `env` and `db` to create an evm
    /// which is then used to transact and trace the transaction.
    fn trace_block_in_place<T, F>(self, transact_and_get_traces: F) -> TracerResult<Vec<T>>
    where
        F: Fn(
            KakarotEvmConfig,
            EnvWithHandlerCfg,
            &mut EthDatabaseSnapshot<P>,
            &reth_rpc_types::Transaction,
        ) -> TracerResult<(Vec<T>, reth_revm::primitives::State)>,
    {
        tokio::task::block_in_place(move || {
            let mut traces = Vec::with_capacity(self.transactions.len());
            let mut transactions = self.transactions.iter().peekable();
            let mut db = self.db;

            while let Some(tx) = transactions.next() {
                // Convert the transaction to an ec recovered transaction and update the env with it
                let tx_ec_recovered = rpc_to_ec_recovered_transaction(tx.clone())?;
                let tx_env = tx_env_with_recovered(&tx_ec_recovered);
                let env = EnvWithHandlerCfg {
                    env: Env::boxed(self.env.env.cfg.clone(), self.env.env.block.clone(), tx_env),
                    handler_cfg: self.env.handler_cfg,
                };

                let (res, state_changes) = transact_and_get_traces(self.cfg.clone(), env, &mut db, tx)?;
                traces.extend(res);

                // Only commit to the database if there are more transactions to process.
                if transactions.peek().is_some() {
                    db.commit(state_changes);
                }
            }

            TracerResult::Ok(traces)
        })
    }
}
