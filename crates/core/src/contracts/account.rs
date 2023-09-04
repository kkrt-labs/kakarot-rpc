use async_trait::async_trait;
use reth_primitives::{Address, Bytes};
use starknet::core::types::{BlockId, FunctionCall, StarknetError};
use starknet::providers::{Provider, ProviderError};
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::{BYTECODE, GET_EVM_ADDRESS, GET_IMPLEMENTATION};
use crate::client::errors::EthApiError;
use crate::client::helpers::{vec_felt_to_bytes, DataDecodingError};
use crate::models::felt::Felt252Wrapper;

#[async_trait]
pub trait Account<'a, P: Provider + Send + Sync + 'a> {
    fn new(starknet_address: FieldElement, provider: &'a P) -> Self;
    fn provider(&self) -> &'a P;
    fn starknet_address(&self) -> FieldElement;
    async fn get_evm_address(&self, starknet_block_id: &BlockId) -> Result<Address, EthApiError<P::Error>> {
        let request = FunctionCall {
            contract_address: self.starknet_address(),
            entry_point_selector: GET_EVM_ADDRESS,
            calldata: vec![],
        };

        let evm_address = self.provider().call(request, starknet_block_id).await?;
        let evm_address: Felt252Wrapper = (*evm_address.first().ok_or_else(|| {
            DataDecodingError::InvalidReturnArrayLength { entrypoint: "get_evm_address".into(), expected: 1, actual: 0 }
        })?)
        .into();

        Ok(evm_address.truncate_to_ethereum_address())
    }

    /// Returns the evm bytecode of the contract.
    async fn bytecode(&self, block_id: &BlockId) -> Result<Bytes, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = vec![];
        let request =
            FunctionCall { contract_address: self.starknet_address(), entry_point_selector: BYTECODE, calldata };

        // Make the function call to get the Starknet contract address
        let bytecode = self.provider().call(request, block_id).await.or_else(|err| match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => Ok(vec![]),
            _ => Err(EthApiError::from(err)),
        })?;

        Ok(vec_felt_to_bytes(bytecode))
    }

    /// Returns the class hash of account implementation of the contract.
    async fn get_implementation(&self, block_id: &BlockId) -> Result<FieldElement, EthApiError<P::Error>> {
        // Prepare the calldata for the get_implementation function call
        let calldata = vec![];
        let request = FunctionCall {
            contract_address: self.starknet_address(),
            entry_point_selector: GET_IMPLEMENTATION,
            calldata,
        };

        // Make the function call to get the Starknet contract address
        let implementation = self.provider().call(request, block_id).await.or_else(|err| match err {
            ProviderError::StarknetError(StarknetError::ContractNotFound) => Ok(vec![]),
            _ => Err(EthApiError::from(err)),
        })?;
        if implementation.len() != 1 {
            return Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "get_implementation".into(),
                expected: 1,
                actual: implementation.len(),
            }
            .into());
        }

        Ok(implementation[0])
    }
}

pub struct KakarotAccount<'a, P> {
    pub starknet_address: FieldElement,
    provider: &'a P,
}

impl<'a, P: Provider + Send + Sync> Account<'a, P> for KakarotAccount<'a, P> {
    fn new(starknet_address: FieldElement, provider: &'a P) -> Self {
        Self { starknet_address, provider }
    }

    fn provider(&self) -> &'a P {
        self.provider
    }

    fn starknet_address(&self) -> FieldElement {
        self.starknet_address
    }
}
