use super::eoa::{TX_GAS_LIMIT, TX_GAS_PRICE};
use crate::{models::felt::Felt252Wrapper, root_project_path};
use alloy_consensus::{TxEip1559, TxLegacy};
use alloy_dyn_abi::{DynSolValue, JsonAbiExt};
use alloy_json_abi::ContractObject;
use alloy_primitives::{TxKind, U256};
use foundry_config::{find_project_root, load_config};
use reth_primitives::Transaction;
use starknet::core::types::Felt;
use std::{fs, path::Path};

#[derive(Clone, Debug)]
pub enum TransactionInfo {
    FeeMarketInfo(TxFeeMarketInfo),
    LegacyInfo(TxLegacyInfo),
}

macro_rules! impl_common_info {
    ($field: ident, $type: ty) => {
        pub const fn $field(&self) -> $type {
            match self {
                TransactionInfo::FeeMarketInfo(info) => info.common.$field,
                TransactionInfo::LegacyInfo(info) => info.common.$field,
            }
        }
    };
}
impl TransactionInfo {
    impl_common_info!(chain_id, Option<u64>);
    impl_common_info!(nonce, u64);
    impl_common_info!(value, u128);
}

#[derive(Clone, Debug, Default)]
pub struct TxCommonInfo {
    pub chain_id: Option<u64>,
    pub nonce: u64,
    pub value: u128,
}

#[derive(Clone, Debug, Default)]
pub struct TxFeeMarketInfo {
    pub common: TxCommonInfo,
    pub max_fee_per_gas: u128,
    pub max_priority_fee_per_gas: u128,
}

#[derive(Clone, Debug, Default)]
pub struct TxLegacyInfo {
    pub common: TxCommonInfo,
    pub gas_price: u128,
}

pub trait EvmContract {
    fn load_contract_bytecode(contract_name: &str) -> Result<ContractObject, eyre::Error> {
        // Construct the path to the compiled JSON file using the root project path and configuration.
        let compiled_path = root_project_path!(Path::new(&load_config().out)
            .join(format!("{contract_name}.sol"))
            .join(format!("{contract_name}.json")));

        // Read the contents of the JSON file into a string.
        let content = fs::read_to_string(compiled_path)?;

        // Deserialize the JSON content into a `ContractObject` and return it.
        Ok(serde_json::from_str(&content)?)
    }

    fn prepare_create_transaction(
        contract_bytecode: &ContractObject,
        constructor_args: &[DynSolValue],
        tx_info: &TxCommonInfo,
    ) -> Result<Transaction, eyre::Error> {
        // Get the ABI from the contract bytecode.
        // Return an error if the ABI is not found.
        let abi = contract_bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;

        // Prepare the deployment data, which includes the bytecode and encoded constructor arguments (if any).
        let deploy_data = match abi.constructor() {
            Some(constructor) => contract_bytecode
                .bytecode
                .clone()
                .unwrap_or_default()
                .into_iter()
                .chain(constructor.abi_encode_input_raw(constructor_args)?)
                .collect(),
            None => contract_bytecode.bytecode.clone().unwrap_or_default().to_vec(),
        };

        // Create and return an EIP-1559 transaction for contract creation.
        Ok(Transaction::Eip1559(TxEip1559 {
            chain_id: tx_info.chain_id.expect("chain id required"),
            nonce: tx_info.nonce,
            gas_limit: TX_GAS_LIMIT,
            max_fee_per_gas: TX_GAS_PRICE.into(),
            input: deploy_data.into(),
            ..Default::default()
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_call_transaction(
        &self,
        selector: &str,
        args: &[DynSolValue],
        tx_info: &TransactionInfo,
    ) -> Result<Transaction, eyre::Error>;
}

#[derive(Default, Debug)]
pub struct KakarotEvmContract {
    pub bytecode: ContractObject,
    pub starknet_address: Felt,
    pub evm_address: Felt,
}

impl KakarotEvmContract {
    pub const fn new(bytecode: ContractObject, starknet_address: Felt, evm_address: Felt) -> Self {
        Self { bytecode, starknet_address, evm_address }
    }
}

impl EvmContract for KakarotEvmContract {
    fn prepare_call_transaction(
        &self,
        selector: &str,
        args: &[DynSolValue],
        tx_info: &TransactionInfo,
    ) -> Result<Transaction, eyre::Error> {
        // Get the ABI from the bytecode.
        // Return an error if the ABI is not found.
        let abi = self.bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;

        // Get the function corresponding to the selector and encode the arguments
        let data = abi
            .function(selector)
            .ok_or_else(|| eyre::eyre!("No function found with selector: {}", selector))
            .and_then(|function| {
            function
                .first()
                .ok_or_else(|| eyre::eyre!("No functions available"))?
                .abi_encode_input(args)
                .map_err(|_| eyre::eyre!("Failed to encode input"))
        })?;

        // Convert the EVM address to a `Felt252Wrapper`.
        let evm_address: Felt252Wrapper = self.evm_address.into();

        // Create the transaction based on the transaction information type.
        let tx = match tx_info {
            TransactionInfo::FeeMarketInfo(fee_market) => Transaction::Eip1559(TxEip1559 {
                chain_id: tx_info.chain_id().expect("chain id required"),
                nonce: tx_info.nonce(),
                gas_limit: TX_GAS_LIMIT,
                to: TxKind::Call(evm_address.try_into()?),
                value: U256::from(tx_info.value()),
                input: data.into(),
                max_fee_per_gas: fee_market.max_fee_per_gas,
                max_priority_fee_per_gas: fee_market.max_priority_fee_per_gas,
                ..Default::default()
            }),
            TransactionInfo::LegacyInfo(legacy) => Transaction::Legacy(TxLegacy {
                chain_id: tx_info.chain_id(),
                nonce: tx_info.nonce(),
                gas_limit: TX_GAS_LIMIT,
                to: TxKind::Call(evm_address.try_into()?),
                value: U256::from(tx_info.value()),
                input: data.into(),
                gas_price: legacy.gas_price,
            }),
        };
        Ok(tx)
    }
}
