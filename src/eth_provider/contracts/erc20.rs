use alloy_sol_types::{sol, SolCall};
use reth_primitives::Address;
use reth_primitives::{BlockId, TxKind, U256};
use reth_rpc_types::request::TransactionInput;
use reth_rpc_types::TransactionRequest;

use crate::eth_provider::error::ExecutionError;
use crate::eth_provider::provider::EthProviderResult;
use crate::eth_provider::provider::EthereumProvider;

sol! {
    #[sol(rpc)]
    contract ERC20Contract {
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
    }
}

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
        // Get the calldata for the function call.
        let calldata = ERC20Contract::balanceOfCall { account: evm_address }.abi_encode();

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
        let balance = U256::try_from_be_slice(&ret)
            .ok_or_else(|| ExecutionError::Other("failed to deserialize balance".to_string()))?;

        Ok(balance)
    }
}
