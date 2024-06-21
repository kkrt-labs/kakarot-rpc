use std::fs;
use std::path::Path;

use alloy_dyn_abi::{DynSolValue, JsonAbiExt};
use alloy_json_abi::ContractObject;
// use ethers::abi::Tokenize;
// use ethers_solc::artifacts::CompactContractBytecode;
use foundry_config::{find_project_root_path, load_config};
use reth_primitives::{Transaction, TxEip1559, TxKind, TxLegacy, U256};
use starknet_crypto::FieldElement;

use crate::models::felt::Felt252Wrapper;
use crate::root_project_path;

use super::eoa::TX_GAS_LIMIT;

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
    // fn load_contract_bytecode(contract_name: &str) -> Result<CompactContractBytecode, eyre::Error> {
    //     let dot_sol = format!("{contract_name}.sol");
    //     let dot_json = format!("{contract_name}.json");

    //     let foundry_default_out = load_config().out;
    //     let compiled_solidity_relative_path = Path::new(&foundry_default_out).join(dot_sol).join(dot_json);
    //     let compiled_solidity_global_path = root_project_path!(&compiled_solidity_relative_path);

    //     println!("compiled_solidity_global_path: {:?}", compiled_solidity_global_path);

    //     let compiled_solidity_file_content = fs::read_to_string(compiled_solidity_global_path)?;
    //     Ok(serde_json::from_str(&compiled_solidity_file_content)?)
    // }

    fn load_contract_bytecode(contract_name: &str) -> Result<ContractObject, eyre::Error> {
        let dot_sol = format!("{contract_name}.sol");
        let dot_json = format!("{contract_name}.json");

        let foundry_default_out = load_config().out;
        let compiled_solidity_relative_path = Path::new(&foundry_default_out).join(dot_sol).join(dot_json);
        let compiled_solidity_global_path = root_project_path!(&compiled_solidity_relative_path);

        println!("compiled_solidity_global_path: {compiled_solidity_global_path:?}");

        let compiled_solidity_file_content = fs::read_to_string(compiled_solidity_global_path)?;
        Ok(serde_json::from_str(&compiled_solidity_file_content)?)
    }

    fn prepare_create_transaction(
        contract_bytecode: &ContractObject,
        constructor_args: &[DynSolValue],
        tx_info: &TxCommonInfo,
    ) -> Result<Transaction, eyre::Error> {
        // // ######################################################
        // // ######################################################
        // // ######################################################
        // // ######################################################
        // use ethers::abi::Token;
        // use ethers::abi::Tokenize;
        // use ethers_solc::artifacts::CompactContractBytecode;

        // let compiled_solidity_file_content = fs::read_to_string(
        //     "/Users/tcoratger/Documents/kakarot/kakarot-rpc/lib/kakarot/solidity_contracts/build/ERC20.sol/ERC20.json",
        // )
        // .unwrap();
        // let compact_contract: CompactContractBytecode = serde_json::from_str(&compiled_solidity_file_content).unwrap();
        // let abi = compact_contract.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;

        // println!("abi avant: {:?}", abi);
        // let bytecode = compact_contract
        //     .bytecode
        //     .as_ref()
        //     .ok_or_else(|| eyre::eyre!("No bytecode found"))?
        //     .object
        //     .as_bytes()
        //     .cloned()
        //     .unwrap_or_default();

        // let constructor_args1 = (
        //     Token::String("Test".into()),               // name
        //     Token::String("TT".into()),                 // symbol
        //     Token::Uint(ethers::types::U256::from(18)), // decimals
        // );

        // let params = constructor_args1.into_tokens();

        // let deploy_data = match abi.constructor() {
        //     Some(constructor) => constructor.encode_input(bytecode.to_vec(), &params)?,
        //     None => bytecode.to_vec(),
        // };

        // println!("deploy data avant: {:?}", deploy_data);

        // // ######################################################
        // // ######################################################
        // // ######################################################

        let abi = contract_bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;

        // println!("abi après: {:?}", abi);
        // let bytecode = contract_bytecode
        //     .bytecode
        //     .as_ref()
        //     .ok_or_else(|| eyre::eyre!("No bytecode found"))?
        //     .object
        //     .as_bytes()
        //     .cloned()
        //     .unwrap_or_default();
        // let params = constructor_args.into_tokens();

        // println!("constructor_args: {:?}", constructor_args);

        let deploy_data = match abi.constructor() {
            // Some(constructor) => constructor.encode_input(bytecode.to_vec(), &params)?,
            // Some(constructor) => constructor.abi_encode_input_raw(constructor_args)?,
            Some(constructor) => contract_bytecode
                .bytecode
                .clone()
                .unwrap_or_default()
                .into_iter()
                .chain(constructor.abi_encode_input_raw(constructor_args)?)
                .collect(),
            // None => bytecode.to_vec(),
            None => contract_bytecode.bytecode.clone().unwrap_or_default().to_vec(),
        };

        // println!("deploy data apres: {:?}", deploy_data);

        Ok(Transaction::Eip1559(TxEip1559 {
            chain_id: tx_info.chain_id.expect("chain id required"),
            nonce: tx_info.nonce,
            gas_limit: TX_GAS_LIMIT,
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
    pub starknet_address: FieldElement,
    pub evm_address: FieldElement,
}

impl KakarotEvmContract {
    pub const fn new(bytecode: ContractObject, starknet_address: FieldElement, evm_address: FieldElement) -> Self {
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
        let abi = self.bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;
        // let params = args.into_tokens();

        // let data = abi.function(selector).and_then(|function| function.encode_input(&params))?;

        // let data = abi.function(selector).and_then(|function| Some(function.first().unwrap().abi_encode_input(args)));

        // // ######################################################
        // // ######################################################
        // // ######################################################
        // // ######################################################
        // use ethers::abi::Token;
        // use ethers::abi::Tokenize;
        // use ethers_solc::artifacts::CompactContractBytecode;

        // let args1 = (
        //     Token::Address(ethers::abi::Address::from_slice(&[
        //         243, 159, 214, 229, 26, 173, 136, 246, 244, 206, 106, 184, 130, 114, 121, 207, 255, 185, 34, 102,
        //     ])),
        //     Token::Uint(ethers::abi::Uint::from_big_endian(&(U256::from(10_000)).to_be_bytes::<32>()[..])),
        // );
        // let params1 = args1.into_tokens();

        // let compiled_solidity_file_content = fs::read_to_string(
        //     "/Users/tcoratger/Documents/kakarot/kakarot-rpc/lib/kakarot/solidity_contracts/build/ERC20.sol/ERC20.json",
        // )
        // .unwrap();
        // let compact_contract: CompactContractBytecode = serde_json::from_str(&compiled_solidity_file_content).unwrap();
        // let abi1 = compact_contract.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;

        // let data = abi1.function(selector).and_then(|function| function.encode_input(&params1)).unwrap();

        // println!("data avant: {:?}", data);

        // // ######################################################
        // // ######################################################
        // // ######################################################

        // println!("selector: {selector}");
        // println!("args: {args:?}");
        // println!("abi.function(selector) : {:?}", abi.function(selector));

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

        // println!("data apres: {:?}", data);

        let evm_address: Felt252Wrapper = self.evm_address.into();

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
