pub mod builder;
mod database;

use self::database::EthDatabaseSnapshot;
use crate::{
    eth_provider::{
        error::{EthApiError, EthereumDataFormatError, TransactionError},
        provider::EthereumProvider,
    },
    tracing::builder::TracingOptions,
};
use eyre::eyre;
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::{ConfigureEvm, ConfigureEvmEnv};
use reth_primitives::{ruint::FromUintError, B256};
use reth_revm::{
    primitives::{CfgEnvWithHandlerCfg, Env, EnvWithHandlerCfg, ExecutionResult, ResultAndState},
    Database, DatabaseCommit,
};
use reth_rpc_eth_types::revm_utils::build_call_evm_env;
use reth_rpc_types::{
    trace::{
        geth::{
            GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingCallOptions, GethDebugTracingOptions,
            GethTrace, TraceResult,
        },
        parity::LocalizedTransactionTrace,
    },
    TransactionInfo, TransactionRequest,
};
use revm_inspectors::tracing::{TracingInspector, TracingInspectorConfig};
use std::collections::HashMap;

pub type TracerResult<T> = Result<T, EthApiError>;

/// Represents the result of tracing a transaction.
type TracingStateResult = TracerResult<(TracingResult, reth_revm::primitives::EvmState)>;

/// Representing the result of tracing transactions.
#[derive(Clone, Debug)]
enum TracingResult {
    /// Geth trace results.
    Geth(Vec<TraceResult>),
    /// Parity trace results.
    Parity(Vec<LocalizedTransactionTrace>),
}

impl TracingResult {
    /// Converts the tracing result into Geth traces.
    const fn as_geth(&self) -> Option<&Vec<TraceResult>> {
        if let Self::Geth(traces) = self {
            Some(traces)
        } else {
            None
        }
    }

    /// Converts the tracing result into Parity traces.
    const fn as_parity(&self) -> Option<&Vec<LocalizedTransactionTrace>> {
        if let Self::Parity(traces) = self {
            Some(traces)
        } else {
            None
        }
    }
    /// Creates a default failure [`TracingResult`] based on the [`TracingOptions`].
    fn default_failure(tracing_options: &TracingOptions, tx: &reth_rpc_types::Transaction) -> Self {
        match tracing_options {
            TracingOptions::Geth(_) | TracingOptions::GethCall(_) => Self::Geth(vec![TraceResult::Success {
                result: GethTrace::Default(reth_rpc_types::trace::geth::DefaultFrame {
                    failed: true,
                    ..Default::default()
                }),
                tx_hash: Some(tx.hash),
            }]),
            TracingOptions::Parity(_) => Self::Parity(
                TracingInspector::default()
                    .into_parity_builder()
                    .into_localized_transaction_traces(TransactionInfo::from(tx)),
            ),
        }
    }
}

#[derive(Debug)]
pub struct Tracer<P: EthereumProvider + Send + Sync> {
    transactions: Vec<reth_rpc_types::Transaction>,
    env: EnvWithHandlerCfg,
    db: EthDatabaseSnapshot<P>,
    tracing_options: TracingOptions,
}

impl<P: EthereumProvider + Send + Sync + Clone> Tracer<P> {
    /// Traces the transaction with Geth tracing options and returns the resulting traces and state.
    fn trace_geth(
        env: EnvWithHandlerCfg,
        db: &mut EthDatabaseSnapshot<P>,
        tx: &reth_rpc_types::Transaction,
        opts: GethDebugTracingOptions,
    ) -> TracingStateResult {
        // Extract options
        let GethDebugTracingOptions { tracer_config, config, tracer, .. } = opts;

        // Check if tracer is provided
        if let Some(tracer) = tracer {
            match tracer {
                // Only support CallTracer for now
                GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer) => {
                    // Convert tracer config to call config
                    let call_config = tracer_config
                        .clone()
                        .into_call_config()
                        .map_err(|err| EthApiError::Transaction(TransactionError::Tracing(err.into())))?;

                    // Initialize tracing inspector with call config
                    let mut inspector =
                        TracingInspector::new(TracingInspectorConfig::from_geth_call_config(&call_config));

                    // Build EVM with environment and inspector
                    let eth_evm_config = EthEvmConfig::default();
                    let evm = eth_evm_config.evm_with_env_and_inspector(db, env, &mut inspector);

                    // Execute transaction
                    let res = transact_in_place(evm)?;

                    // Get call traces
                    let call_frame = inspector.into_geth_builder().geth_call_traces(
                        tracer_config.into_call_config().map_err(|err| TransactionError::Tracing(err.into()))?,
                        res.result.gas_used(),
                    );

                    // Return success trace result
                    return Ok((
                        TracingResult::Geth(vec![TraceResult::Success {
                            result: call_frame.into(),
                            tx_hash: Some(tx.hash),
                        }]),
                        res.state,
                    ));
                }
                // Return error for unsupported tracers
                _ => {
                    return Err(EthApiError::Transaction(TransactionError::Tracing(
                        eyre!("only call tracer is currently supported").into(),
                    )))
                }
            }
        }

        // Use default tracer
        let mut inspector = TracingInspector::new(TracingInspectorConfig::from_geth_config(&config));
        let eth_evm_config = EthEvmConfig::default();
        let evm = eth_evm_config.evm_with_env_and_inspector(db, env, &mut inspector);
        let res = transact_in_place(evm)?;
        let gas_used = res.result.gas_used();
        let return_value = res.result.into_output().unwrap_or_default();
        let frame = inspector.into_geth_builder().geth_traces(gas_used, return_value, config);
        Ok((
            TracingResult::Geth(vec![TraceResult::Success { result: frame.into(), tx_hash: Some(tx.hash) }]),
            res.state,
        ))
    }

    /// Traces the transaction with Parity tracing options and returns the resulting traces and state.
    fn trace_parity(
        env: EnvWithHandlerCfg,
        db: &mut EthDatabaseSnapshot<P>,
        tx: &reth_rpc_types::Transaction,
        tracing_config: TracingInspectorConfig,
    ) -> TracingStateResult {
        // Get block base fee
        let block_base_fee = env
            .env
            .block
            .basefee
            .try_into()
            .map_err(|err: FromUintError<u128>| TransactionError::Tracing(err.into()))?;

        // Initialize tracing inspector with given config
        let mut inspector = TracingInspector::new(tracing_config);

        // Build EVM with environment and inspector
        let eth_evm_config = EthEvmConfig::default();
        let evm = eth_evm_config.evm_with_env_and_inspector(db, env, &mut inspector);

        // Execute transaction
        let res = transact_in_place(evm)?;

        // Create transaction info
        let transaction_info = TransactionInfo::from(tx).with_base_fee(block_base_fee);

        // Return Parity trace result
        Ok((
            TracingResult::Parity(inspector.into_parity_builder().into_localized_transaction_traces(transaction_info)),
            res.state,
        ))
    }

    /// Trace the block in the parity format.
    pub fn trace_block(self) -> TracerResult<Option<Vec<LocalizedTransactionTrace>>> {
        let txs = self.transactions.clone();
        Ok(Some(self.trace_transactions(TracingResult::as_parity, &txs)?))
    }

    /// Returns the debug trace in the Geth.
    /// Currently only supports the call tracer or the default tracer.
    pub fn debug_block(self) -> TracerResult<Vec<TraceResult>> {
        let txs = self.transactions.clone();
        self.trace_transactions(TracingResult::as_geth, &txs)
    }

    pub fn debug_transaction(mut self, transaction_hash: B256) -> TracerResult<GethTrace> {
        for tx in self.transactions.clone() {
            if tx.hash == transaction_hash {
                // We only want to trace the transaction with the given hash.
                let trace = self
                    .trace_transactions(TracingResult::as_geth, &[tx])?
                    .first()
                    .cloned()
                    .ok_or(TransactionError::Tracing(eyre!("No trace found").into()))?;
                return match trace {
                    TraceResult::Success { result, .. } => Ok(result),
                    TraceResult::Error { error, .. } => Err(TransactionError::Tracing(error.into()).into()),
                };
            }

            let env = env_with_tx(&self.env, tx.clone())?;
            let eth_evm_config = EthEvmConfig::default();
            transact_commit_in_place(eth_evm_config.evm_with_env(&mut self.db, env))?;
        }

        Err(EthApiError::TransactionNotFound(transaction_hash))
    }

    /// Debugs a transaction request by tracing it using the provided tracing options.
    ///
    /// This function returns an error if the tracing options are not supported or if there is an issue
    /// with the EVM environment or transaction execution.
    pub fn debug_transaction_request(self, request: &TransactionRequest) -> TracerResult<GethTrace> {
        // Attempt to get Geth tracing options from the provided tracing options.
        let opts = self
            .tracing_options
            .as_geth_call()
            .ok_or_else(|| {
                // Return an error if the tracing options are not supported.
                EthApiError::Transaction(TransactionError::Tracing(
                    eyre!("only `GethDebugTracingCallOptions` tracing options are supported for call tracing").into(),
                ))
            })?
            .clone();

        // Extract the tracing options from the obtained Geth tracing options.
        let GethDebugTracingCallOptions { tracing_options, .. } = opts;
        let GethDebugTracingOptions { tracer, tracer_config, .. } = tracing_options;

        // Check if a tracer is provided.
        if let Some(tracer) = tracer {
            match tracer {
                // Only support CallTracer for now.
                GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer) => {
                    // Build the EVM environment using the provided configuration and request.

                    let env = build_call_evm_env(
                        CfgEnvWithHandlerCfg { cfg_env: self.env.cfg.clone(), handler_cfg: self.env.handler_cfg },
                        self.env.block.clone(),
                        request.clone(),
                    )?;

                    // Convert the tracer configuration into call configuration.
                    let call_config =
                        tracer_config.into_call_config().map_err(|err| TransactionError::Tracing(err.into()))?;

                    // Create a new tracing inspector with the call configuration.
                    let mut inspector =
                        TracingInspector::new(TracingInspectorConfig::from_geth_call_config(&call_config));

                    // Build EVM with environment and inspector.
                    let eth_evm_config = EthEvmConfig::default();
                    let evm = eth_evm_config.evm_with_env_and_inspector(self.db, env, &mut inspector);

                    // Execute the transaction.
                    let res = transact_in_place(evm)?;

                    // Get the call traces from the inspector.
                    let frame = inspector.into_geth_builder().geth_call_traces(call_config, res.result.gas_used());

                    // Return the obtained call traces.
                    return Ok(frame.into());
                }

                // Return an error for unsupported tracers.
                _ => {
                    return Err(EthApiError::Transaction(TransactionError::Tracing(
                        eyre!("only call tracer is currently supported").into(),
                    )))
                }
            }
        }

        // Return a default Geth trace if no tracer is provided.
        Ok(GethTrace::Default(Default::default()))
    }

    /// Traces the provided transactions using the given closure.
    /// The `convert_result` closure takes the resulting tracing result
    /// and converts it into the desired type.
    fn trace_transactions<T: Clone>(
        self,
        convert_result: fn(&TracingResult) -> Option<&Vec<T>>,
        transactions: &[reth_rpc_types::Transaction],
    ) -> TracerResult<Vec<T>> {
        let mut traces: Vec<T> = Vec::with_capacity(self.transactions.len());
        let mut transactions = transactions.iter().peekable();
        let mut db = self.db;

        while let Some(tx) = transactions.next() {
            let env = env_with_tx(&self.env, tx.clone())?;

            let (res, state_changes) =
                if tx.other.get("isRunOutOfResources").and_then(serde_json::Value::as_bool).unwrap_or(false) {
                    (TracingResult::default_failure(&self.tracing_options, tx), HashMap::default())
                } else {
                    match &self.tracing_options {
                        TracingOptions::Geth(opts) => Self::trace_geth(env, &mut db, tx, opts.clone())?,
                        TracingOptions::Parity(tracing_config) => {
                            Self::trace_parity(env, &mut db, tx, *tracing_config)?
                        }
                        TracingOptions::GethCall(_) => {
                            return Err(EthApiError::Transaction(TransactionError::Tracing(
                                eyre!("`TracingOptions::GethCall` is not supported in `trace_transactions` context")
                                    .into(),
                            )))
                        }
                    }
                };

            if let Some(result) = convert_result(&res) {
                result.iter().for_each(|item| traces.push(item.clone()));
            }

            // Only commit to the database if there are more transactions to process.
            if transactions.peek().is_some() {
                db.commit(state_changes);
            }
        }

        TracerResult::Ok(traces)
    }
}

/// Returns the environment with the transaction env updated to the given transaction.
fn env_with_tx(env: &EnvWithHandlerCfg, tx: reth_rpc_types::Transaction) -> TracerResult<EnvWithHandlerCfg> {
    // Convert the transaction to an ec recovered transaction and update the env with it.
    let tx_ec_recovered = tx.try_into().map_err(|_| EthereumDataFormatError::TransactionConversion)?;

    let tx_env = EthEvmConfig::default().tx_env(&tx_ec_recovered);

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
    use crate::eth_provider::{database::Database, provider::EthDataProvider};
    use builder::TracerBuilder;
    use mongodb::options::{DatabaseOptions, ReadConcern, WriteConcern};
    use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
    use std::{str::FromStr, sync::Arc};
    use url::Url;

    #[tokio::test(flavor = "multi_thread")]
    #[ignore = "this test is used for debugging purposes only"]
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
                    .read_concern(ReadConcern::majority())
                    .write_concern(WriteConcern::majority())
                    .build(),
            ),
        );

        let eth_provider = Arc::new(EthDataProvider::new(db, starknet_provider).await.unwrap());
        let tracer = TracerBuilder::new(eth_provider)
            .await
            .unwrap()
            .with_transaction_hash(B256::from_str("INSERT THE TRANSACTION HASH YOU WISH TO DEBUG").unwrap())
            .await
            .unwrap()
            .with_tracing_options(TracingInspectorConfig::default_parity().into())
            .build()
            .unwrap();

        // When
        let _ = tracer.trace_block().unwrap();
    }
}
