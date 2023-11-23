use reth_primitives::U256;
use starknet::core::types::{BlockId, FunctionCall};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::BALANCE_OF;
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::models::felt::Felt252Wrapper;

/// Abstraction for a Starknet ERC20 contract.
pub struct StarknetErc20<'a, P> {
    pub address: FieldElement,
    provider: &'a P,
}

impl<'a, P: Provider + Send + Sync> StarknetErc20<'a, P> {
    pub const fn new(provider: &'a P, address: FieldElement) -> Self {
        Self { provider, address }
    }

    pub async fn balance_of(&self, starknet_address: &FieldElement, block_id: &BlockId) -> Result<U256, EthApiError> {
        // Prepare the calldata for the bytecode function call
        let calldata = vec![*starknet_address];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: BALANCE_OF, calldata };

        // Make the function call to get the account balance
        let result = self.provider.call(request, block_id).await?;
        if result.len() != 2 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "balance_of".into(),
                expected: 2,
                actual: result.len(),
            }
            .into());
        };
        let low: Felt252Wrapper = result[0].into(); // safe indexing
        let high: Felt252Wrapper = result[1].into(); // safe indexing

        let value = Into::<U256>::into(low) + (Into::<U256>::into(high) << 128);
        Ok(value)
    }
}
