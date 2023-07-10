use std::marker::PhantomData;

use starknet::core::types::{BlockId, FunctionCall};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::constants::selectors::{BALANCE_OF, BYTECODE, COMPUTE_STARKNET_ADDRESS, ETH_CALL, GET_EVM_ADDRESS};
use crate::client::constants::STARKNET_NATIVE_TOKEN;
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;

pub struct KakarotContract<P> {
    pub address: FieldElement,
    pub proxy_account_class_hash: FieldElement,
    _phantom: PhantomData<P>,
}

impl<P: Provider + Send + Sync> KakarotContract<P> {
    #[must_use]
    pub fn new(address: FieldElement, proxy_account_class_hash: FieldElement) -> Self {
        Self { address, proxy_account_class_hash, _phantom: PhantomData }
    }

    pub async fn balance(
        &self,
        provider: &P,
        starknet_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        // Prepare the calldata for the balanceOf function call
        let calldata = vec![*starknet_address];
        let request = FunctionCall {
            contract_address: FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(),
            entry_point_selector: BALANCE_OF,
            calldata,
        };

        // Make the function call to get the balance
        let result = provider.call(request, block_id).await?;
        Ok(*result.first().ok_or_else(|| DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "balance".into(),
            expected: 1,
            actual: 0,
        })?)
    }

    pub async fn bytecode(
        &self,
        provider: &P,
        starknet_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<Vec<FieldElement>, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let request =
            FunctionCall { contract_address: *starknet_address, entry_point_selector: BYTECODE, calldata: vec![] };

        Ok(provider.call(request, block_id).await?)
    }

    pub async fn compute_starknet_address(
        &self,
        provider: &P,
        eth_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        // Prepare the calldata for the compute_starknet_address function call
        let calldata = vec![*eth_address];
        let request =
            FunctionCall { contract_address: self.address, entry_point_selector: COMPUTE_STARKNET_ADDRESS, calldata };

        // Make the function call to get the Starknet contract address
        let result = provider.call(request, block_id).await?;
        match result.first() {
            Some(x) if result.len() == 1 => Ok(*x),
            _ => Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "compute_starknet_address".into(),
                expected: 1,
                actual: 0,
            }
            .into()),
        }
    }

    pub async fn eth_call(
        &self,
        provider: &P,
        eth_address: &FieldElement,
        eth_calldata: &mut Vec<FieldElement>,
        block_id: &BlockId,
    ) -> Result<Vec<FieldElement>, EthApiError<P::Error>> {
        let mut calldata =
            vec![*eth_address, FieldElement::MAX, FieldElement::ZERO, FieldElement::ZERO, eth_calldata.len().into()];

        calldata.append(eth_calldata);

        let request = FunctionCall { contract_address: self.address, entry_point_selector: ETH_CALL, calldata };
        Ok(provider.call(request, block_id).await?)
    }

    pub async fn get_evm_address(
        &self,
        provider: &P,
        starknet_address: &FieldElement,
        block_id: &BlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        let request = FunctionCall {
            contract_address: *starknet_address,
            entry_point_selector: GET_EVM_ADDRESS,
            calldata: vec![],
        };

        let result = provider.call(request, block_id).await?;
        match result.first() {
            Some(x) if result.len() == 1 => Ok(*x),
            _ => Err(DataDecodingError::InvalidReturnArrayLength {
                entrypoint: "get_evm_address".into(),
                expected: 1,
                actual: 0,
            }
            .into()),
        }
    }
}
