//! Transaction validation logic.

use reth_primitives::ChainSpec;
use reth_transaction_pool::BlobStore;
use std::sync::Arc;

/// A wrapper around the Reth [reth_transaction_pool::validate::EthTransactionValidatorBuilder].
/// The produced Validator will reject EIP4844 transactions not supported by Kakarot at the moment.

#[derive(Debug)]
pub struct EthTransactionValidatorBuilder(reth_transaction_pool::validate::EthTransactionValidatorBuilder);

impl EthTransactionValidatorBuilder {
    /// Create a new [EthTransactionValidatorBuilder].
    pub fn new(chain_spec: Arc<ChainSpec>) -> Self {
        Self(reth_transaction_pool::validate::EthTransactionValidatorBuilder::new(chain_spec))
    }

    /// Build the [EthTransactionValidator]. Force `no_eip4844`.
    pub fn build<Client, S>(
        self,
        client: Client,
        store: S,
    ) -> reth_transaction_pool::validate::EthTransactionValidator<Client, S>
    where
        S: BlobStore,
    {
        let builder = self.0.no_eip4844();
        builder.build(client, store)
    }
}
