// use ethers::abi::AbiEncode;
// use ethers::core::types::Address as EthersAddress;
// use ethers::prelude::abigen;
use reth_primitives::Address;

use reth_primitives::{BlockId, TxKind, U256};
use reth_rpc_types::request::TransactionInput;
use reth_rpc_types::TransactionRequest;

use crate::eth_provider::error::{ExecutionError, KakarotError};
use crate::eth_provider::provider::EthProviderResult;
use crate::eth_provider::provider::EthereumProvider;
use crate::eth_rpc::config::RPCConfig;

use alloy_provider::ProviderBuilder;
use alloy_sol_types::sol;

// // abigen generates a lot of unused code, needs to be benchmarked if performances ever become a
// // concern
// abigen!(
//     IERC20,
//     r#"[
//         function balanceOf(address account) external view returns (uint256)
//         function allowance(address owner, address spender) external view returns (uint256)
//     ]"#,
// );

sol! {
    #[sol(rpc)]
    contract MyContract {
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
        // // ######################################################
        // // ######################################################
        // // ######################################################

        // // Prepare the calldata for the bytecode function call
        // let address = EthersAddress::from_slice(evm_address.as_slice());
        // let calldata1 = IERC20Calls::BalanceOf(BalanceOfCall { account: address }).encode();

        // println!("calldata avant: {:?}", calldata1);

        // // ######################################################
        // // ######################################################
        // // ######################################################

        // Prepare the calldata for the bytecode function call
        let provider = ProviderBuilder::new()
            .on_builtin(&RPCConfig::from_env().expect("Failed to load Kakarot RPC config").socket_addr)
            .await
            .expect("Failed to create provider via alloy `ProviderBuilder`");
        let call_builder = MyContract::new(Address::ZERO, provider);
        let function_call = call_builder.balanceOf(evm_address);
        let calldata = function_call.calldata();

        // println!("calldata apres: {:?}", calldata.to_vec());

        let request = TransactionRequest {
            from: Some(Address::default()),
            to: Some(TxKind::Call(self.address)),
            gas_price: Some(0),
            gas: Some(1_000_000),
            value: Some(U256::ZERO),
            input: TransactionInput { input: Some(calldata.clone()), data: None },
            ..Default::default()
        };

        let ret = self.provider.call(request, Some(block_id)).await?;

        // println!("ret: {:?}", ret);
        let balance = U256::try_from_be_slice(&ret)
            .ok_or_else(|| KakarotError::from(ExecutionError::Other("failed to deserialize balance".to_string())))?;

        Ok(balance)
    }
}
