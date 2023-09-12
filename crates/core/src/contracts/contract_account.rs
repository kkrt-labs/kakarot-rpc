use reth_primitives::U256;
use starknet::core::types::{BlockId, FunctionCall};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use super::account::Account;
use crate::client::constants::selectors::{GET_NONCE, STORAGE};
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::models::felt::Felt252Wrapper;

/// Abstraction for a Kakarot contract account.
pub struct ContractAccount<'a, P> {
    pub address: FieldElement,
    provider: &'a P,
}

impl<'a, P: Provider + Send + Sync> Account<'a, P> for ContractAccount<'a, P> {
    fn new(address: FieldElement, provider: &'a P) -> Self {
        Self { address, provider }
    }

    fn provider(&self) -> &'a P {
        self.provider
    }

    fn starknet_address(&self) -> FieldElement {
        self.address
    }
}

impl<'a, P: Provider + Send + Sync> ContractAccount<'a, P> {
    /// Returns the value stored at the given key in the evm contract storage. Not to be confused
    /// with the Starknet contract storage.
    pub async fn storage(
        &self,
        key_low: &FieldElement,
        key_high: &FieldElement,
        block_id: &BlockId,
    ) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the storage function call
        let calldata = vec![*key_low, *key_high];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: STORAGE, calldata };

        // Make the function call to get the Starknet contract address
        let result = self.provider.call(request, block_id).await?;
        if result.len() != 2 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "storage".into(),
                expected: 2,
                actual: result.len(),
            }
            .into());
        }
        let low: Felt252Wrapper = result[0].into(); // safe indexing
        let high: Felt252Wrapper = result[1].into(); // safe indexing

        let value = Into::<U256>::into(low) + (Into::<U256>::into(high) << 128);
        Ok(value)
    }

    /// Returns the nonce of the contract account.
    /// In Kakarot EVM, there are two types of accounts: EOA and Contract Account.
    /// EOA nonce is handled by Starknet protocol.
    /// Contract Account nonce is handled by Kakarot through a dedicated storage, this function
    /// returns that storage value.
    pub async fn nonce(&self, block_id: &BlockId) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the get_nonce function call
        let calldata = vec![];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: GET_NONCE, calldata };

        let result = self.provider.call(request, block_id).await?;
        if result.len() != 1 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "get_nonce".into(),
                expected: 1,
                actual: result.len(),
            }
            .into());
        }

        Ok(Into::<Felt252Wrapper>::into(result[0]).into())
    }
}
