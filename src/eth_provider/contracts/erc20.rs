#![allow(clippy::pub_underscore_fields)]

use crate::eth_provider::{
    error::ExecutionError,
    provider::{EthProviderResult, EthereumProvider},
};
use alloy_dyn_abi::DynSolType;
use alloy_sol_types::{sol, SolCall};
use reth_primitives::{Address, BlockId, Bytes, TxKind, U256};
use reth_rpc_types::{request::TransactionInput, TransactionRequest};

sol! {
    #[sol(rpc)]
    contract ERC20Contract {
        function balanceOf(address account) external view returns (uint256);
        function allowance(address owner, address spender) external view returns (uint256);
        function decimals() external view returns (uint8);
        function name() external view returns (string);
        function symbol() external view returns (string);
    }
}

/// Abstraction for a Kakarot ERC20 contract.
#[derive(Debug)]
pub struct EthereumErc20<P: EthereumProvider> {
    /// The address of the ERC20 contract.
    pub address: Address,
    /// The provider for interacting with the Ethereum network.
    pub provider: P,
}

impl<P: EthereumProvider> EthereumErc20<P> {
    /// Creates a new instance of [`EthereumErc20`].
    pub const fn new(address: Address, provider: P) -> Self {
        Self { address, provider }
    }

    /// Gets the balance of the specified address.
    pub async fn balance_of(&self, evm_address: Address, block_id: BlockId) -> EthProviderResult<U256> {
        // Encode the calldata for the balanceOf function call
        let calldata = ERC20Contract::balanceOfCall { account: evm_address }.abi_encode();
        // Call the contract with the encoded calldata
        let ret = self.call_contract(calldata, block_id).await?;
        // Deserialize the returned bytes into a U256 balance
        let balance = U256::try_from_be_slice(&ret)
            .ok_or_else(|| ExecutionError::Other("failed to deserialize balance".to_string()))?;
        Ok(balance)
    }

    /// Gets the number of decimals the token uses.
    pub async fn decimals(&self, block_id: BlockId) -> EthProviderResult<U256> {
        // Encode the calldata for the decimals function call
        let calldata = ERC20Contract::decimalsCall {}.abi_encode();
        // Call the contract with the encoded calldata
        let ret = self.call_contract(calldata, block_id).await?;
        // Deserialize the returned bytes into a U256 representing decimals
        let decimals = U256::try_from_be_slice(&ret)
            .ok_or_else(|| ExecutionError::Other("failed to deserialize decimals".to_string()))?;
        Ok(decimals)
    }

    /// Gets the name of the token.
    pub async fn name(&self, block_id: BlockId) -> EthProviderResult<String> {
        // Encode the calldata for the name function call
        let calldata = ERC20Contract::nameCall {}.abi_encode();
        // Call the contract with the encoded calldata
        let ret = self.call_contract(calldata, block_id).await?;
        // Deserialize the returned bytes into a String representing the name
        let name = DynSolType::String
            .abi_decode(&ret)
            .map_err(|_| ExecutionError::Other("failed to deserialize name".to_string()))?;
        Ok(name.as_str().unwrap_or_default().to_string())
    }

    /// Gets the symbol of the token.
    pub async fn symbol(&self, block_id: BlockId) -> EthProviderResult<String> {
        // Encode the calldata for the symbol function call
        let calldata = ERC20Contract::symbolCall {}.abi_encode();
        // Call the contract with the encoded calldata
        let ret = self.call_contract(calldata, block_id).await?;
        // Deserialize the returned bytes into a String representing the symbol
        let symbol = DynSolType::String
            .abi_decode(&ret)
            .map_err(|_| ExecutionError::Other("failed to deserialize symbol".to_string()))?;
        Ok(symbol.as_str().unwrap_or_default().to_string())
    }

    /// Gets the allowance the owner has granted to the spender.
    pub async fn allowance(&self, owner: Address, spender: Address, block_id: BlockId) -> EthProviderResult<U256> {
        // Encode the calldata for the allowance function call
        let calldata = ERC20Contract::allowanceCall { owner, spender }.abi_encode();
        // Call the contract with the encoded calldata
        let ret = self.call_contract(calldata, block_id).await?;
        // Deserialize the returned bytes into a U256 representing the allowance
        let allowance = U256::try_from_be_slice(&ret)
            .ok_or_else(|| ExecutionError::Other("failed to deserialize allowance".to_string()))?;
        Ok(allowance)
    }

    /// Calls the contract with the given calldata.
    async fn call_contract(&self, calldata: Vec<u8>, block_id: BlockId) -> EthProviderResult<Bytes> {
        self.provider
            .call(
                TransactionRequest {
                    from: Some(Address::default()),
                    to: Some(TxKind::Call(self.address)),
                    gas_price: Some(0),
                    gas: Some(1_000_000),
                    value: Some(U256::ZERO),
                    input: TransactionInput { input: Some(calldata.into()), data: None },
                    ..Default::default()
                },
                Some(block_id),
            )
            .await
    }
}
