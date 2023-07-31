use ethers::abi::{AbiParser, Contract, Token};
use ethers::types::Address;
use lazy_static::lazy_static;
use reth_primitives::{BlockId, U256};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::contracts::kakarot::KakarotContract;
use crate::models::block::EthBlockId;

lazy_static! {
    /// Abi linkage for Erc20 functions and events
    static ref ABI: Contract = AbiParser::default().parse_str(r#"[
        function totalSupply() external view returns (uint256)
        function balanceOf(address account) external view returns (uint256)
        function transfer(address recipient, uint256 amount) external returns (bool)
        function allowance(address owner, address spender) external view returns (uint256)
        function approve(address spender, uint256 amount) external returns (bool)
        function transferFrom( address sender, address recipient, uint256 amount) external returns (bool)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event Approval(address indexed owner, address indexed spender, uint256 value)
    ]"#).unwrap();
}

/// Abstraction for a Kakarot ERC20 contract.
pub struct EthereumErc20<'a, P> {
    pub address: FieldElement,
    kakarot_contract: &'a KakarotContract<P>,
}

impl<'a, P: Provider + Send + Sync> EthereumErc20<'a, P> {
    pub fn new(address: FieldElement, kakarot_contract: &'a KakarotContract<P>) -> Self {
        Self { address, kakarot_contract }
    }

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = ABI
            .function("balanceOf")
            .map_err(|err| EthApiError::ContractError(err.into()))?
            .encode_input(&[Token::Address(evm_address)])
            .map_err(|err| EthApiError::ContractError(err.into()))?;
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
}
