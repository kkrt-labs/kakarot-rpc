use crate::{
    into_via_wrapper,
    providers::eth_provider::{
        error::ExecutionError,
        starknet::{ERC20Reader, STARKNET_ETH, STARKNET_NATIVE_TOKEN},
        utils::{class_hash_not_declared, contract_not_found},
    },
};
use alloy_primitives::U256;
use starknet::{
    core::types::{BlockId, Felt},
    providers::Provider,
};
use std::ops::Deref;
use tracing::Instrument;

/// A provider wrapper around the Starknet provider to expose utility methods.
#[derive(Debug, Clone)]
pub struct StarknetProvider<SP: Provider + Send + Sync> {
    /// The underlying Starknet provider wrapped in an [`Arc`] for shared ownership across threads.
    provider: SP,
}

impl<SP: Provider + Send + Sync> Deref for StarknetProvider<SP> {
    type Target = SP;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

impl<SP> StarknetProvider<SP>
where
    SP: Provider + Send + Sync,
{
    /// Creates a new [`StarknetProvider`] instance from a Starknet provider.
    pub const fn new(provider: SP) -> Self {
        Self { provider }
    }

    /// Retrieves the balance of the parent token for a Starknet address.
    ///
    /// # Arguments
    /// * `token_address` - The address of the token contract
    /// * `address` - The address to check the balance for
    /// * `block_id` - The block ID at which to check the balance
    ///
    /// # Returns
    /// * `Result<U256, ExecutionError>` - The balance as a U256 value or an execution error
    pub async fn eth_balance_at(&self, address: Felt, block_id: BlockId) -> Result<U256, ExecutionError> {
        self.balance_at(STARKNET_ETH, address, block_id).await
    }

    /// Retrieves the balance of the native Starknet token for an address.
    ///
    /// # Arguments
    /// * `token_address` - The address of the token contract
    /// * `address` - The address to check the balance for
    /// * `block_id` - The block ID at which to check the balance
    ///
    /// # Returns
    /// * `Result<U256, ExecutionError>` - The balance as a U256 value or an execution error
    pub async fn strk_balance_at(&self, address: Felt, block_id: BlockId) -> Result<U256, ExecutionError> {
        self.balance_at(STARKNET_NATIVE_TOKEN, address, block_id).await
    }

    /// Internal method to retrieve the balance of any ERC20 token for a Starknet address.
    ///
    /// # Arguments
    /// * `token_address` - The address of the ERC20 token contract
    /// * `address` - The address to check the balance for
    /// * `block_id` - The block ID at which to check the balance
    ///
    /// # Returns
    /// * `Result<U256, ExecutionError>` - The balance as a U256 value or an execution error
    ///
    /// # Notes
    /// - Returns a balance of 0 if the contract is not deployed or the class hash is not declared
    /// - The balance is returned as a U256, combining low and high 128-bit components
    async fn balance_at(&self, token_address: Felt, address: Felt, block_id: BlockId) -> Result<U256, ExecutionError> {
        // Create a new `ERC20Reader` instance for the Starknet parent native token
        let eth_contract = ERC20Reader::new(token_address, &self.provider);

        // Call the `balanceOf` method on the contract for the given address and block ID, awaiting the result
        let span = tracing::span!(tracing::Level::INFO, "sn::balance");
        let res = eth_contract.balanceOf(&address).block_id(block_id).call().instrument(span).await;

        // Check if the contract was not found or the class hash not declared,
        // returning a default balance of 0 if true.
        if contract_not_found(&res) || class_hash_not_declared(&res) {
            return Ok(Default::default());
        }
        // Otherwise, extract the balance from the result, converting any errors to ExecutionError
        let balance = res.map_err(ExecutionError::from)?.balance;

        // Convert the low and high parts of the balance to U256
        let low: U256 = into_via_wrapper!(balance.low);
        let high: U256 = into_via_wrapper!(balance.high);

        // Combine the low and high parts to form the final balance and return it
        Ok(low + (high << 128))
    }
}
