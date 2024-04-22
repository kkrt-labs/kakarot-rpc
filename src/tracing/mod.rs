mod database;

use crate::{
    eth_provider::{
        error::{EthApiError, EthereumDataFormatError, TransactionError},
        provider::EthereumProvider,
    },
    models::transaction::rpc_to_ec_recovered_transaction,
};
use reth_primitives::{
    revm::env::tx_env_with_recovered,
    revm_primitives::{BlockEnv, CfgEnv, Env, EnvWithHandlerCfg, SpecId},
    TxType, B256,
};
use reth_revm::tracing::{TracingInspector, TracingInspectorConfig};
use reth_revm::{primitives::HandlerCfg, DatabaseCommit};
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

        let block = maybe_block.unwrap();
        if block.header.hash.unwrap_or_default().is_zero() {
            return Err(EthApiError::UnknownBlock);
        }

        // DB should use the state of the parent block
        let parent_block_hash = block.header.parent_hash;
        let db = database::EthDatabaseSnapshot::new(self.eth_provider.clone(), BlockId::Hash(parent_block_hash.into()));

        let env = Box::new(self.setup_env(block.header.clone()).await);
        let ctx = reth_revm::Context::new(reth_revm::EvmContext::new_with_env(db, env.clone()), self.inspector.clone());

        let env = EnvWithHandlerCfg::new(env, HandlerCfg::new(SpecId::CANCUN));
        let mut handler = reth_revm::Handler::new(env.handler_cfg);
        handler.append_handler_register_plain(reth_revm::inspector_handle_register);
        let mut evm = reth_revm::Evm::new(ctx, handler);

        let traces = tokio::task::block_in_place(move || {
            let transactions = match &block.transactions {
                BlockTransactions::Full(transactions) => transactions,
                _ => return Err(TransactionError::ExpectedFullTransactions.into()),
            };
            let mut transactions = transactions.iter().peekable();
            let mut traces = Vec::with_capacity(block.transactions.len());

            while let Some(tx) = transactions.next() {
                // Convert the transaction to an ec recovered transaction and update the evm env with it
                let tx_ec_recovered = rpc_to_ec_recovered_transaction(tx.clone())?;
                let tx_env = tx_env_with_recovered(&tx_ec_recovered);
                evm = evm.modify().modify_tx_env(|env| *env = tx_env).build();

                // Transact the transaction
                let result = evm.transact().map_err(|err| TransactionError::Tracing(err.into()))?;

                // Convert the traces to a parity trace and accumulate them
                let parity_builder = evm.context.external.clone().into_parity_builder();

                // Extract the base fee from the transaction based on the transaction type
                let base_fee = match tx.transaction_type.and_then(|typ| typ.to::<u8>().try_into().ok()) {
                    Some(TxType::Legacy) | Some(TxType::Eip2930) => tx.gas_price,
                    Some(TxType::Eip1559) => tx
                        .max_fee_per_gas
                        .map(|fee| fee.saturating_sub(tx.max_priority_fee_per_gas.unwrap_or_default())),
                    _ => return Err(EthereumDataFormatError::TransactionConversionError.into()),
                }
                .map(|fee| fee.try_into())
                .transpose()
                .map_err(|_| EthereumDataFormatError::PrimitiveError)?;

                let transaction_info = TransactionInfo {
                    hash: Some(tx.hash),
                    index: tx.transaction_index.map(|i| i.to()),
                    block_hash: tx.block_hash,
                    block_number: tx.block_number.map(|bn| bn.to()),
                    base_fee,
                };
                traces.extend(parity_builder.into_localized_transaction_traces(transaction_info));

                // Only commit to the database if there are more transactions to process.
                if transactions.peek().is_some() {
                    evm.context.evm.inner.db.commit(result.state);
                }

                // Update the evm env with a new inspector
                evm = evm.modify().modify_external_context(|insp| *insp = self.inspector.clone()).build();
            }

            Result::<_, EthApiError>::Ok(traces)
        })?;

        Ok(Some(traces))
    }

    async fn setup_env(&self, block_header: Header) -> Env {
        let mut env = self.env.clone();

        let Header { number, timestamp, gas_limit, miner, base_fee_per_gas, difficulty, .. } = block_header;
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
