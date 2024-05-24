use std::fs;
use std::path::Path;

use ethers::abi::Tokenize;
use ethers_solc::artifacts::CompactContractBytecode;
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
    fn load_contract_bytecode(contract_name: &str) -> Result<CompactContractBytecode, eyre::Error> {
        let dot_sol = format!("{contract_name}.sol");
        let dot_json = format!("{contract_name}.json");

        let foundry_default_out = load_config().out;
        let compiled_solidity_relative_path = Path::new(&foundry_default_out).join(dot_sol).join(dot_json);
        let compiled_solidity_global_path = root_project_path!(&compiled_solidity_relative_path);

        let compiled_solidity_file_content = fs::read_to_string(compiled_solidity_global_path)?;
        Ok(serde_json::from_str(&compiled_solidity_file_content)?)
    }

    fn prepare_create_transaction<T: Tokenize>(
        contract_bytecode: &CompactContractBytecode,
        constructor_args: T,
        tx_info: &TxCommonInfo,
    ) -> Result<Transaction, eyre::Error> {
        let abi = contract_bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;
        let bytecode = contract_bytecode
            .bytecode
            .as_ref()
            .ok_or_else(|| eyre::eyre!("No bytecode found"))?
            .object
            .as_bytes()
            .cloned()
            .unwrap_or_default();
        let params = constructor_args.into_tokens();

        let deploy_data = match abi.constructor() {
            Some(constructor) => constructor.encode_input(bytecode.to_vec(), &params)?,
            None => bytecode.to_vec(),
        };

        Ok(Transaction::Eip1559(TxEip1559 {
            chain_id: tx_info.chain_id.expect("chain id required"),
            nonce: tx_info.nonce,
            gas_limit: TX_GAS_LIMIT,
            input: deploy_data.into(),
            ..Default::default()
        }))
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_call_transaction<T: Tokenize>(
        &self,
        selector: &str,
        args: T,
        tx_info: &TransactionInfo,
    ) -> Result<Transaction, eyre::Error>;
}

#[derive(Default, Debug)]
pub struct KakarotEvmContract {
    pub bytecode: CompactContractBytecode,
    pub starknet_address: FieldElement,
    pub evm_address: FieldElement,
}

impl KakarotEvmContract {
    pub const fn new(
        bytecode: CompactContractBytecode,
        starknet_address: FieldElement,
        evm_address: FieldElement,
    ) -> Self {
        Self { bytecode, starknet_address, evm_address }
    }
}

impl EvmContract for KakarotEvmContract {
    fn prepare_call_transaction<T: Tokenize>(
        &self,
        selector: &str,
        args: T,
        tx_info: &TransactionInfo,
    ) -> Result<Transaction, eyre::Error> {
        let abi = self.bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;
        let params = args.into_tokens();

        let data = abi.function(selector).and_then(|function| function.encode_input(&params))?;

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
