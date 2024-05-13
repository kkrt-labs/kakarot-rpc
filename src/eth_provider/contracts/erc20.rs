use ethers::abi::AbiEncode;
use ethers::core::types::Address as EthersAddress;
use ethers::prelude::abigen;
use reth_primitives::Address;

use reth_primitives::{BlockId, TxKind, U256};
use reth_rpc_types::request::TransactionInput;
use reth_rpc_types::TransactionRequest;

use crate::eth_provider::error::KakarotError;
use crate::eth_provider::provider::EthProviderResult;
use crate::eth_provider::provider::EthereumProvider;

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
#[derive(Debug)]
pub struct EthereumErc20<P: EthereumProvider> {
    pub address: Address,
    pub provider: P,
}

impl<P: EthereumProvider> EthereumErc20<P> {
    pub const fn new(address: Address, provider: P) -> Self {
        Self { address, provider }
    }

    pub async fn balance_of(self, evm_address: Address, block_id: BlockId) -> EthProviderResult<U256> {
        // Prepare the calldata for the bytecode function call
        let address = EthersAddress::from_slice(evm_address.as_slice());
        let calldata = IERC20Calls::BalanceOf(BalanceOfCall { account: address }).encode();

        let request = TransactionRequest {
            from: Some(Address::default()),
            to: Some(TxKind::Call(self.address)),
            gas_price: Some(0),
            gas: Some(1_000_000),
            value: Some(U256::ZERO),
            input: TransactionInput { input: Some(calldata.into()), data: None },
            ..Default::default()
        };

        let ret = self.provider.call(request, Some(block_id)).await?;
        let balance = U256::try_from_be_slice(&ret).ok_or(KakarotError::CallError(
            cainome::cairo_serde::Error::Deserialize("failed to deserialize balance".to_string()),
        ))?;

        Ok(balance)
    }
}
