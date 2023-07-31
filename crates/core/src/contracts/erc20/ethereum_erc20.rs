use ethers::abi::{Function, Token};
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

const BALANCE_OF_ABI: &str = r#"{ "constant": true, "inputs": [ { "name": "_owner", "type": "address" } ], "name": "balanceOf", "outputs": [ { "name": "balance", "type": "uint256" } ], "payable": false, "stateMutability": "view", "type": "function" }"#;
lazy_static! {
    /// Abi linkage for Erc20 function BalanceOf
    static ref BALANCE_OF: Function = serde_json::from_str(BALANCE_OF_ABI).unwrap();
}

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

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        // Prepare the calldata for the bytecode function call
        let calldata = BALANCE_OF
            .encode_input(&[Token::Address(evm_address)])
            .map_err(|x| EthApiError::ContractError(x.into()))?;
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
