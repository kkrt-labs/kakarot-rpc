use std::sync::Arc;

use async_trait::async_trait;
use reth_primitives::{Address, Bytes};
use starknet::core::types::{BlockId, FunctionCall, StarknetError};
use starknet::providers::{MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage};
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::{BYTECODE, GET_EVM_ADDRESS, GET_IMPLEMENTATION};
use crate::client::errors::EthApiError;
use crate::client::helpers::{vec_felt_to_bytes, DataDecodingError};
use crate::models::felt::Felt252Wrapper;

#[async_trait]
pub trait Account<P: Provider + Send + Sync> {
    fn new(starknet_address: FieldElement, provider: Arc<P>) -> Self;
    fn provider(&self) -> Arc<P>;
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
            ProviderError::StarknetError(StarknetErrorWithMessage {
                code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                ..
            }) => Ok(vec![]),
            _ => Err(EthApiError::from(err)),
        })?;

        if bytecode.is_empty() {
            return Ok(Bytes::default());
        }

        // bytecode_len is the first element of the returned array
        // TODO: Remove Manual Decoding
        Ok(vec_felt_to_bytes(bytecode[1..].to_vec()))
    }

    /// Returns the class hash of account implementation of the contract.
    async fn implementation(&self, block_id: &BlockId) -> Result<FieldElement, EthApiError<P::Error>> {
        // Prepare the calldata for the get_implementation function call
        let calldata = vec![];
        let request = FunctionCall {
            contract_address: self.starknet_address(),
            entry_point_selector: GET_IMPLEMENTATION,
            calldata,
        };

        // Make the function call to get the Starknet contract address
        let class_hash = self.provider().call(request, block_id).await?;
        let class_hash = *class_hash.first().ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "get_implementation".into(),
            expected: 1,
            actual: 0,
        })?;
        Ok(class_hash)
    }
}

pub struct KakarotAccount<P> {
    pub starknet_address: FieldElement,
    provider: Arc<P>,
}

impl<P: Provider + Send + Sync + 'static> Account<P> for KakarotAccount<P> {
    fn new(starknet_address: FieldElement, provider: Arc<P>) -> Self {
        Self { starknet_address, provider }
    }

    fn provider(&self) -> Arc<P> {
        self.provider.clone()
    }

    fn starknet_address(&self) -> FieldElement {
        self.starknet_address
    }
}
