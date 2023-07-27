use std::marker::PhantomData;

use reth_primitives::U256;
use starknet::core::types::{BlockId, FunctionCall};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::BALANCE_OF;
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::models::felt::Felt252Wrapper;

pub struct Erc20Contract<P> {
    pub address: FieldElement,
    _phantom: PhantomData<P>,
}

impl<P: Provider + Send + Sync> Erc20Contract<P> {
    #[must_use]
    pub fn new(address: FieldElement) -> Self {
        Self { address, _phantom: PhantomData }
    }

    pub async fn balance_of(
        &self,
        starknet_provider: &P,
        starknet_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = vec![*starknet_address];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: BALANCE_OF, calldata };

        // Make the function call to get the account balance
        let result = starknet_provider.call(request, block_id).await?;
        if result.len() != 2 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "balance_of".into(),
                expected: 2,
                actual: result.len(),
            }
            .into());
        };
        let low: Felt252Wrapper = (*result.get(0).unwrap()).into(); // safe unwrap
        let high: Felt252Wrapper = (*result.get(1).unwrap()).into(); // safe unwrap

        let value = Into::<U256>::into(low) + (Into::<U256>::into(high) << 128);
        Ok(value)
    }
}
