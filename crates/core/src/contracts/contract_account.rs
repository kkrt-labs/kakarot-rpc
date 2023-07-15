use std::marker::PhantomData;

use reth_primitives::{Bytes, U256};
use starknet::core::types::{BlockId, FunctionCall, StarknetError};
use starknet::providers::{Provider, ProviderError};
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::{BYTECODE, STORAGE};
use crate::client::errors::EthApiError;
use crate::client::helpers::{vec_felt_to_bytes, DataDecodingError};
use crate::models::felt::Felt252Wrapper;

pub struct ContractAccount<P> {
    pub address: FieldElement,
    _phantom: PhantomData<P>,
}

impl<P: Provider + Send + Sync> ContractAccount<P> {
    #[must_use]
    pub fn new(address: FieldElement) -> Self {
        Self { address, _phantom: PhantomData }
    }

    pub async fn bytecode(&self, starknet_provider: &P, block_id: &BlockId) -> Result<Bytes, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = vec![];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: BYTECODE, calldata };

        // Make the function call to get the Starknet contract address
        let bytecode = starknet_provider.call(request, block_id).await.or_else(|err| match err {
            ProviderError::StarknetError(starknet_error) => match starknet_error {
                // TODO: we just need to test against ContractNotFound but madara is currently returning the wrong
                // error See https://github.com/keep-starknet-strange/madara/issues/853
                StarknetError::ContractError | StarknetError::ContractNotFound => Ok(vec![]),
                _ => Err(EthApiError::from(err)),
            },
            _ => Err(EthApiError::from(err)),
        })?;

        Ok(vec_felt_to_bytes(bytecode))
    }

    pub async fn storage(
        &self,
        starknet_provider: &P,
        key_low: &FieldElement,
        key_high: &FieldElement,
        block_id: &BlockId,
    ) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the storage function call
        let calldata = vec![*key_low, *key_high];
        let request = FunctionCall { contract_address: self.address, entry_point_selector: STORAGE, calldata };

        // Make the function call to get the Starknet contract address
        let result = starknet_provider.call(request, block_id).await?;
        if result.len() != 2 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "storage".into(),
                expected: 2,
                actual: result.len(),
            }
            .into());
        }
        let low: Felt252Wrapper = (*result.get(0).unwrap()).into(); // safe unwrap
        let high: Felt252Wrapper = (*result.get(1).unwrap()).into(); // safe unwrap

        let value = Into::<U256>::into(low) + (Into::<U256>::into(high) << 128);
        Ok(value)
    }
}
