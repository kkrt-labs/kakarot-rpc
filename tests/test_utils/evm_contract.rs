use std::fs;
use std::path::Path;

use ethers::abi::Tokenize;
use ethers_solc::artifacts::CompactContractBytecode;
use foundry_config::{find_project_root_path, load_config};
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::starknet_client::constants::CHAIN_ID;
use reth_primitives::{Transaction, TransactionKind, TxEip1559};
use starknet_crypto::FieldElement;

use crate::root_project_path;

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
        nonce: u64,
    ) -> Result<Transaction, eyre::Error>;

    fn prepare_call_transaction<T: Tokenize>(
        &self,
        selector: &str,
        constructor_args: T,
        nonce: u64,
        value: u128,
    ) -> Result<Transaction, eyre::Error>;
}

#[derive(Default)]
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
    fn prepare_create_transaction<T: Tokenize>(
        contract_bytecode: &CompactContractBytecode,
        constructor_args: T,
        nonce: u64,
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
            chain_id: CHAIN_ID,
            nonce,
            max_priority_fee_per_gas: Default::default(),
            max_fee_per_gas: Default::default(),
            gas_limit: u64::MAX,
            to: TransactionKind::Create,
            value: 0,
            input: deploy_data.into(),
            access_list: Default::default(),
        }))
    }

    fn prepare_call_transaction<T: Tokenize>(
        &self,
        selector: &str,
        args: T,
        nonce: u64,
        value: u128,
    ) -> Result<Transaction, eyre::Error> {
        let abi = self.bytecode.abi.as_ref().ok_or_else(|| eyre::eyre!("No ABI found"))?;
        let params = args.into_tokens();

        let data = abi.function(selector).and_then(|function| function.encode_input(&params))?;

        let evm_address: Felt252Wrapper = self.evm_address.try_into()?;

        Ok(Transaction::Eip1559(TxEip1559 {
            chain_id: CHAIN_ID,
            nonce,
            max_priority_fee_per_gas: Default::default(),
            max_fee_per_gas: Default::default(),
            gas_limit: u64::MAX,
            to: TransactionKind::Call(evm_address.try_into()?),
            value,
            input: data.into(),
            access_list: Default::default(),
        }))
    }
}
