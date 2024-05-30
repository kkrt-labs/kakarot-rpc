pub mod builder;
mod config;
mod database;

use eyre::eyre;
use reth_primitives::revm::env::tx_env_with_recovered;
use reth_primitives::ruint::FromUintError;
use reth_primitives::B256;
use reth_revm::primitives::{Env, EnvWithHandlerCfg, ExecutionResult, ResultAndState};
use reth_revm::{Database, DatabaseCommit};
use reth_rpc_types::trace::geth::{GethTrace, TraceResult};
use reth_rpc_types::{
    trace::{
        geth::{GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions},
        parity::LocalizedTransactionTrace,
    },
    TransactionInfo,
};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};

use self::config::EvmBuilder;
use self::database::EthDatabaseSnapshot;
use crate::eth_provider::{
    error::{EthApiError, EthereumDataFormatError, TransactionError},
    provider::EthereumProvider,
};

pub type TracerResult<T> = Result<T, EthApiError>;

#[derive(Debug)]
pub struct Tracer<P: EthereumProvider + Send + Sync> {
    transactions: Vec<reth_rpc_types::Transaction>,
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
            |env: EnvWithHandlerCfg,
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
                let evm = EvmBuilder::evm_with_env_and_inspector(db, env, &mut inspector);
                let res = transact_in_place(evm)?;

                let parity_builder = inspector.into_parity_builder();

                let mut transaction_info = TransactionInfo::from(tx);
                transaction_info.base_fee = Some(block_base_fee);

                Ok((parity_builder.into_localized_transaction_traces(transaction_info), res.state))
            };

        let txs = self.transactions.clone();
        let traces = self.trace_transactions(transact_to_parity_trace, &txs)?;

        Ok(Some(traces))
    }

    /// Returns the debug trace in the Geth.
    /// Currently only supports the call tracer or the default tracer.
    pub fn debug_block(self, opts: GethDebugTracingOptions) -> TracerResult<Vec<TraceResult>> {
        let transact_to_geth_trace = transact_and_get_traces_geth(opts);
        let txs = self.transactions.clone();
        let traces = self.trace_transactions(transact_to_geth_trace, &txs)?;

        Ok(traces)
    }

    pub fn debug_transaction(
        mut self,
        transaction_hash: B256,
        opts: GethDebugTracingOptions,
    ) -> TracerResult<GethTrace> {
        for tx in self.transactions.clone() {
            if tx.hash == transaction_hash {
                let transact_and_get_traces = transact_and_get_traces_geth::<P>(opts);
                // We only want to trace the transaction with the given hash.
                let trace = self
                    .trace_transactions(transact_and_get_traces, &[tx])?
                    .first()
                    .cloned()
                    .ok_or(TransactionError::Tracing(eyre!("No trace found").into()))?;
                return match trace {
                    TraceResult::Success { result, .. } => Ok(result),
                    TraceResult::Error { error, .. } => Err(TransactionError::Tracing(error.into()).into()),
                };
            }

            let env = env_with_tx(&self.env, tx.clone())?;
            let evm = EvmBuilder::evm_with_env(&mut self.db, env);
            transact_commit_in_place(evm)?;
        }

        Err(EthApiError::TransactionNotFound)
    }

    /// Traces the provided transactions using the given closure.
    /// The function `transact_and_get_traces` closure uses the `env` and `db` to create an evm
    /// which is then used to transact and trace the transaction.
    fn trace_transactions<T, F>(
        self,
        transact_and_get_traces: F,
        transactions: &[reth_rpc_types::Transaction],
    ) -> TracerResult<Vec<T>>
    where
        F: Fn(
            EnvWithHandlerCfg,
            &mut EthDatabaseSnapshot<P>,
            &reth_rpc_types::Transaction,
        ) -> TracerResult<(Vec<T>, reth_revm::primitives::State)>,
    {
        let mut traces = Vec::with_capacity(self.transactions.len());
        let mut transactions = transactions.iter().peekable();
        let mut db = self.db;

        while let Some(tx) = transactions.next() {
            let env = env_with_tx(&self.env, tx.clone())?;

            let (res, state_changes) = transact_and_get_traces(env, &mut db, tx)?;
            traces.extend(res);

            // Only commit to the database if there are more transactions to process.
            if transactions.peek().is_some() {
                db.commit(state_changes);
            }
        }

        TracerResult::Ok(traces)
    }
}

/// Returns a closure that transacts and gets the geth traces for the given transaction. Captures the
/// `opts` in order to use it in the closure.
fn transact_and_get_traces_geth<P: EthereumProvider + Send + Sync + Clone>(
    opts: GethDebugTracingOptions,
) -> impl Fn(
    EnvWithHandlerCfg,
    &mut EthDatabaseSnapshot<P>,
    &reth_rpc_types::Transaction,
) -> TracerResult<(Vec<TraceResult>, reth_revm::primitives::State)> {
    move |env: EnvWithHandlerCfg,
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
                    let evm = EvmBuilder::evm_with_env_and_inspector(db, env, &mut inspector);

                    let res = transact_in_place(evm)?;
                    let call_frame = inspector.into_geth_builder().geth_call_traces(
                        tracer_config.into_call_config().map_err(|err| TransactionError::Tracing(err.into()))?,
                        res.result.gas_used(),
                    );
                    Ok((vec![TraceResult::Success { result: call_frame.into(), tx_hash: Some(tx.hash) }], res.state))
                }
                _ => Err(EthApiError::Transaction(TransactionError::Tracing(
                    eyre!("only call tracer is currently supported").into(),
                ))),
            };
        }

        // default tracer
        let mut inspector = TracingInspector::new(TracingInspectorConfig::from_geth_config(&config));
        let evm = EvmBuilder::evm_with_env_and_inspector(db, env, &mut inspector);

        let res = transact_in_place(evm)?;
        let gas_used = res.result.gas_used();
        let return_value = res.result.into_output().unwrap_or_default();
        let frame = inspector.into_geth_builder().geth_traces(gas_used, return_value, config);
        Ok((vec![TraceResult::Success { result: frame.into(), tx_hash: Some(tx.hash) }], res.state))
    }
}

/// Returns the environment with the transaction env updated to the given transaction.
fn env_with_tx(env: &EnvWithHandlerCfg, tx: reth_rpc_types::Transaction) -> TracerResult<EnvWithHandlerCfg> {
    // Convert the transaction to an ec recovered transaction and update the env with it.
    let tx_ec_recovered = tx.try_into().map_err(|_| EthereumDataFormatError::TransactionConversionError)?;

    let tx_env = tx_env_with_recovered(&tx_ec_recovered);
    Ok(EnvWithHandlerCfg {
        env: Env::boxed(env.env.cfg.clone(), env.env.block.clone(), tx_env),
        handler_cfg: env.handler_cfg,
    })
}

/// Runs the `evm.transact_commit()` in a blocking context using `tokio::task::block_in_place`.
/// This is needed in order to enter a blocking context which is then converted to a async
/// context in the implementation of [Database] using `Handle::current().block_on(async { ... })`
/// ⚠️ `evm.transact()` should NOT be used as is and we should always make use of the `transact_in_place` function
fn transact_in_place<I, DB: Database>(mut evm: reth_revm::Evm<'_, I, DB>) -> TracerResult<ResultAndState>
where
    <DB as Database>::Error: std::error::Error + Sync + Send + 'static,
{
    tokio::task::block_in_place(|| evm.transact().map_err(|err| TransactionError::Tracing(err.into()).into()))
}

/// Runs the `evm.transact_commit()` in a blocking context using `tokio::task::block_in_place`.
/// This is needed in order to enter a blocking context which is then converted to a async
/// context in the implementation of [Database] using `Handle::current().block_on(async { ... })`
/// ⚠️ `evm.transact_commit()` should NOT be used as is and we should always make use of the `transaction_commit_in_place` function
fn transact_commit_in_place<I, DB: Database + DatabaseCommit>(
    mut evm: reth_revm::Evm<'_, I, DB>,
) -> TracerResult<ExecutionResult>
where
    <DB as Database>::Error: std::error::Error + Sync + Send + 'static,
{
    tokio::task::block_in_place(|| evm.transact_commit().map_err(|err| TransactionError::Tracing(err.into()).into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth_provider::database::Database;
    use crate::eth_provider::provider::EthDataProvider;
    use builder::TracerBuilder;
    use hex::FromHex;
    use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
    use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
    use std::sync::Arc;
    use url::Url;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "This test is used for debugging purposes only"]
    async fn test_debug_tracing() {
        // Set the env vars
        std::env::set_var("KAKAROT_ADDRESS", "CHECK THE KAKAROT ADDRESS FOR THE BLOCK YOU ARE DEBUGGING");
        std::env::set_var(
            "UNINITIALIZED_ACCOUNT_CLASS_HASH",
            "CHECK THE KAKAROT UNINITIALIZED ACCOUNT CLASS HASH FOR THE BLOCK YOU ARE DEBUGGING",
        );

        // Given
        let url = Url::parse("https://juno-kakarot-dev.karnot.xyz/").unwrap();
        let starknet_provider = JsonRpcClient::new(HttpTransport::new(url));

        // Start a local mongodb instance with the state of the network:
        // - Install `mongod`.
        // - Run `brew services start mongodb-community` on MacOS.
        // - Connect to the remote mongodb instance using MongoCompass and export the headers collection
        //   and the transactions collection. Instructions for exporting/importing can be found at
        //   `https://www.mongodb.com/docs/compass/current/import-export/`.
        // - Connect to the local mongodb instance using MongoCompass.
        // - Import the headers and transactions collections.
        // - ‼️ You might need to manually fix some transactions that don't have an `accessList` field. ‼️
        // - ‼️ Be sure to import the collections in the database called `local`. ‼️
        let db_client = mongodb::Client::with_uri_str("mongodb://localhost:27017/").await.unwrap();
        let db = Database::new(
            db_client.database_with_options(
                "local",
                DatabaseOptions::builder()
                    .read_concern(ReadConcern::MAJORITY)
                    .write_concern(WriteConcern::MAJORITY)
                    .build(),
            ),
        );

        let eth_provider = Arc::new(EthDataProvider::new(db, starknet_provider).await.unwrap());
        let tracer = TracerBuilder::new(eth_provider)
            .await
            .unwrap()
            .with_transaction_hash(B256::from_hex("INSERT THE TRANSACTION HASH YOU WISH TO DEBUG").unwrap())
            .await
            .unwrap()
            .build()
            .unwrap();

        // When
        let _ = tracer.trace_block(TracingInspectorConfig::default_parity()).unwrap();
    }
}
