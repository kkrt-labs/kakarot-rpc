use anyhow::Result;
use starknet::{
    core::types::{ExecutionResult, Felt, StarknetError},
    providers::{Provider, ProviderError},
};
use std::time::Duration;
use tracing::info;

/// Code taken from
/// <https://github.com/xJonathanLEI/starkli/blob/42c7cfc42102e399f76896ebbbc5291393f40d7e/src/utils.rs#L13>
/// Credits to Jonathan Lei
pub async fn watch_tx<P>(
    provider: P,
    transaction_hash: Felt,
    poll_interval: Duration,
    count: usize,
) -> Result<()>
where
    P: Provider,
{
    let mut i = 0usize;
    loop {
        if i >= count {
            return Err(anyhow::anyhow!("transaction not confirmed after {} tries", count));
        }
        match provider.get_transaction_receipt(transaction_hash).await {
            Ok(receipt) => match receipt.execution_result() {
                ExecutionResult::Succeeded => {
                    info!("Transaction confirmed successfully 🎉");
                    return Ok(());
                }
                ExecutionResult::Reverted { reason } => {
                    return Err(anyhow::anyhow!("transaction reverted: {}", reason));
                }
            },
            Err(ProviderError::StarknetError(StarknetError::TransactionHashNotFound)) => {
                info!("Transaction not confirmed yet...");
            }
            // Some nodes are still serving error code `25` for tx hash not found. This is
            // technically a bug on the node's side, but we maximize compatibility here by also
            // accepting it.
            Err(err) => return Err(err.into()),
        }

        tokio::time::sleep(poll_interval).await;
        i += 1;
    }
}
