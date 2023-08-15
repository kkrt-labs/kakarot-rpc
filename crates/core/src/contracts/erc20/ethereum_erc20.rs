use ethers::abi::AbiEncode;
use ethers::prelude::abigen;
use ethers::types::Address;
use reth_primitives::{BlockId, U256, U8};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::contracts::kakarot::KakarotContract;
use crate::models::block::EthBlockId;

// abigen generates a lot of unused code, needs to be benchmarked if performances ever become a
// concern
abigen!(
    IERC20,
    r#"[
        function balanceOf(address account) external view returns (uint256)
        function allowance(address owner, address spender) external view returns (uint256)
        function name() external view returns (string)
        function symbol() external view returns (string)
        function decimals() external view returns (uint8)
    ]"#,
);

/// Abstraction for a Kakarot ERC20 contract.
pub struct EthereumErc20<'a, P> {
    pub address: FieldElement,
    kakarot_contract: &'a KakarotContract<P>,
}

impl<'a, P: Provider + Send + Sync> EthereumErc20<'a, P> {
    pub fn new(address: FieldElement, kakarot_contract: &'a KakarotContract<P>) -> Self {
        Self { address, kakarot_contract }
    }
    pub async fn allowance(
        self,
        account_address: Address,
        spender_address: Address,
        block_id: BlockId,
    ) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata =
            IERC20Calls::Allowance(AllowanceCall { owner: account_address, spender: spender_address }).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let allowance: Vec<u8> = result.0.into();

        Ok(U256::try_from_be_slice(allowance.as_slice()).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "allowance".into(),
            expected: 32,
            actual: allowance.len(),
        })?)
    }

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::BalanceOf(BalanceOfCall { account: evm_address }).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let balance: Vec<u8> = result.0.into();

        Ok(U256::try_from_be_slice(balance.as_slice()).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "balanceOf".into(),
            expected: 32,
            actual: balance.len(),
        })?)
    }

    pub async fn name(self, block_id: BlockId) -> Result<String, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::Symbol(SymbolCall).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let name: Vec<u8> = result.0.into();

        Ok(String::from_utf8(name.clone()).map_err(|_| DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "name".into(),
            expected: 32,
            actual: name.len(),
        })?)
    }

    pub async fn symbol(self, block_id: BlockId) -> Result<String, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::Symbol(SymbolCall).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let symbol: Vec<u8> = result.0.into();

        Ok(String::from_utf8(symbol.clone()).map_err(|_| DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "symbol".into(),
            expected: 32,
            actual: symbol.len(),
        })?)
    }

    pub async fn decimals(self, block_id: BlockId) -> Result<U8, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::Decimals(DecimalsCall).encode();
        let calldata = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let result = self.kakarot_contract.eth_call(&self.address, calldata, &block_id).await?;
        let decimals: Vec<u8> = result.0.into();

        Ok(U8::try_from_be_slice(decimals.as_slice()).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "decimals".into(),
            expected: 32,
            actual: decimals.len(),
        })?)
    }
}
