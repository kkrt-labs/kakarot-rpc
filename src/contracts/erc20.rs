use std::sync::Arc;

use anyhow::anyhow;
use ethers::abi::AbiEncode;
use ethers::prelude::abigen;
use ethers::types::Address;
use reth_primitives::{BlockId, U256};
use starknet::core::types::BlockId as StarknetBlockId;
use starknet::macros::felt;
use starknet::providers::Provider;
use starknet_abigen_parser::cairo_types::CairoArrayLegacy;
use starknet_crypto::FieldElement;

use crate::contracts::kakarot_contract::KakarotCoreReader;
use crate::models::block::EthBlockId;
use crate::models::felt::Felt252Wrapper;
use crate::starknet_client::constants::TX_ORIGIN_ZERO;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::helpers::DataDecodingError;

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
pub struct EthereumErc20<P> {
    pub address: FieldElement,
    pub provider: Arc<P>,
    pub kakarot_address: FieldElement,
}

impl<P: Provider + Send + Sync> EthereumErc20<P> {
    pub const fn new(address: FieldElement, provider: Arc<P>, kakarot_address: FieldElement) -> Self {
        Self { address, provider, kakarot_address }
    }

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> Result<U256, EthApiError> {
        // Prepare the calldata for the bytecode function call
        let calldata = IERC20Calls::BalanceOf(BalanceOfCall { account: evm_address }).encode();
        let calldata: Vec<_> = calldata.into_iter().map(FieldElement::from).collect();

        let block_id = EthBlockId::new(block_id);
        let block_id: StarknetBlockId = block_id.try_into()?;

        let origin = Felt252Wrapper::from(*TX_ORIGIN_ZERO);

        let kakarot_reader = KakarotCoreReader::new(self.kakarot_address, &self.provider);

        let gas_limit = felt!("0x100000");
        let gas_price = felt!("0x1");
        let value = FieldElement::ZERO;

        let (_, return_data, success) = kakarot_reader
            .eth_call(
                &origin.into(),
                &self.address,
                &gas_limit,
                &gas_price,
                &value,
                &calldata.len().into(),
                &CairoArrayLegacy(calldata),
            )
            .block_id(block_id)
            .call()
            .await?;

        if success == FieldElement::ZERO {
            let revert_reason =
                return_data.0.into_iter().filter_map(|x| u8::try_from(x).ok()).map(|x| x as char).collect::<String>();
            return Err(EthApiError::Other(anyhow!("Revert reason: {}", revert_reason)));
        }

        let balance = return_data.0.into_iter().filter_map(|x: FieldElement| u8::try_from(x).ok()).collect::<Vec<_>>();

        Ok(U256::try_from_be_slice(&balance).ok_or(DataDecodingError::InvalidReturnArrayLength {
            entrypoint: "balanceOf".into(),
            expected: 32,
            actual: balance.len(),
        })?)
    }
}
