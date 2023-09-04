use ethers::abi::AbiEncode;
use ethers::prelude::abigen;
use ethers::types::Address;
use reth_primitives::{BlockId, U256};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::constants::TX_ORIGIN_ZERO;
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::contracts::kakarot::KakarotContract;
use crate::models::block::EthBlockId;
use crate::models::felt::Felt252Wrapper;

// abigen generates a lot of unused code, needs to be benchmarked if performances ever become a
// concern
abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
    ]"#,
);

/// Abstraction for a Kakarot ERC20 contract.
pub struct EthereumErc20<'a, P> {
    pub address: FieldElement,
    kakarot_contract: &'a KakarotContract<P>,
}

impl<'a, P: Provider + Send + Sync + 'static> EthereumErc20<'a, P> {
    pub fn new(address: FieldElement, kakarot_contract: &'a KakarotContract<P>) -> Self {
        Self { address, kakarot_contract }
    }

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::BalanceOf(BalanceOfCall { account: evm_address }).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let origin = Felt252Wrapper::from(*TX_ORIGIN_ZERO);

        let result = self.kakarot_contract.eth_call(&origin.into(), &self.address, calldata, &block_id).await?;
        let balance: Vec<u8> = result.0.into();

        Ok(U256::try_from_be_slice(balance.as_slice()).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "balanceOf".into(),
            expected: 32,
            actual: balance.len(),
        })?)
    }
}
