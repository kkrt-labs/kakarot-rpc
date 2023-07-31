use ethers::utils::id;
use reth_primitives::{Bytes, U256};
use starknet::core::types::BlockId;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::contracts::kakarot::KakarotContract;
use crate::models::felt::Felt252Wrapper;

/// Abstraction for a Kakarot ERC20 contract.
pub struct EthereumErc20<'a, P> {
    pub address: FieldElement,
    kakarot_contract: &'a KakarotContract<P>,
}

impl<'a, P: Provider + Send + Sync> EthereumErc20<'a, P> {
    #[must_use]
    pub fn new(address: FieldElement, kakarot_contract: &'a KakarotContract<P>) -> Self {
        Self { address, kakarot_contract }
    }

    pub async fn balance_of(self, evm_address: FieldElement, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        let entrypoint = &id("balanceOf(address)");
        let evm_address: Felt252Wrapper = evm_address.into();

        // Prepare the calldata for the bytecode function call
        let mut calldata = entrypoint.to_vec();
        calldata.append(&mut Into::<Bytes>::into(evm_address).to_vec());
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let balance: Vec<u8> = result.0.into();

        Ok(U256::try_from_be_slice(balance.as_slice()).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "balanceOf".into(),
            expected: 32,
            actual: balance.len(),
        })?)
    }
}
