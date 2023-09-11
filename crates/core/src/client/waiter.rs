use std::sync::Arc;
use std::time::Duration;

use starknet::core::types::{MaybePendingTransactionReceipt, StarknetError, TransactionReceipt, TransactionStatus};
use starknet::providers::{MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage};
use starknet_crypto::FieldElement;
use tokio::time::Instant;

use super::errors::EthApiError;

/// A helper struct for waiting for a transaction to be mined.
/// Inspired by https://github.com/dojoengine/dojo/blob/main/crates/dojo-world/src/utils.rs
pub struct TransactionWaiter<P: Provider> {
    provider: Arc<P>,
    transaction_hash: FieldElement,
    interval: Duration,
    timeout: Duration,
}

impl<P: Provider> TransactionWaiter<P> {
    pub fn new(
        provider: Arc<P>,
        transaction_hash: FieldElement,
        interval_in_milli: u64,
        timeout_in_milli: u64,
    ) -> Self {
        Self {
            provider,
            transaction_hash,
            interval: Duration::from_millis(interval_in_milli),
            timeout: Duration::from_millis(timeout_in_milli),
        }
    }

    pub fn with_transaction_hash(&mut self, tx_hash: FieldElement) -> &mut Self {
        self.transaction_hash = tx_hash;
        self
    }

    pub async fn poll(&self) -> Result<(), EthApiError<P::Error>> {
        let started_at = Instant::now();
        loop {
            let elapsed = started_at.elapsed();
            if elapsed > self.timeout {
                return Err(EthApiError::RequestError(ProviderError::StarknetError(StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                    message: "Transaction waiter timed out".to_string(),
                })));
            }

            let receipt = self.provider.get_transaction_receipt(self.transaction_hash).await;
            match receipt {
                Ok(receipt) => match receipt {
                    MaybePendingTransactionReceipt::Receipt(receipt) => match transaction_receipt_status(receipt) {
                        TransactionStatus::AcceptedOnL2 | TransactionStatus::AcceptedOnL1 => {
                            return Ok(());
                        }
                        TransactionStatus::Rejected => {
                            return Err(EthApiError::Other(anyhow::anyhow!(
                                "Pooling Failed: the transaction {} has been rejected",
                                self.transaction_hash
                            )));
                        }
                        _ => (),
                    },
                    MaybePendingTransactionReceipt::PendingReceipt(_) => (),
                },
                Err(error) => {
                    match error {
                        ProviderError::StarknetError(StarknetErrorWithMessage {
                            code: MaybeUnknownErrorCode::Known(StarknetError::TransactionHashNotFound),
                            ..
                        }) => {
                            // do nothing because to comply with json-rpc spec, even in case of
                            // TransactionStatus::Received an error will be returned, so we should
                            // continue polling see: https://github.com/xJonathanLEI/starknet-rs/blob/832c6cdc36e5899cf2c82f8391a4dd409650eed1/starknet-providers/src/sequencer/provider.rs#L134C1-L134C1
                        }
                        _ => {
                            return Err(error.into());
                        }
                    }
                }
            };
            tokio::time::sleep(self.interval).await;
        }
    }
}

fn transaction_receipt_status(receipt: TransactionReceipt) -> TransactionStatus {
    match receipt {
        TransactionReceipt::Invoke(receipt) => receipt.status,
        TransactionReceipt::Declare(receipt) => receipt.status,
        TransactionReceipt::Deploy(receipt) => receipt.status,
        TransactionReceipt::DeployAccount(receipt) => receipt.status,
        TransactionReceipt::L1Handler(receipt) => receipt.status,
    }
}
