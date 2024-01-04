use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Error as IoError;
use std::path::Path;

use crate::models::felt::Felt252Wrapper;
use crate::starknet_client::constants::STARKNET_NATIVE_TOKEN;
use eyre::Result;
use foundry_config::find_project_root_path;
use lazy_static::lazy_static;
use reth_primitives::{Address, Bytes, H256, U256, U64};
use serde::{Deserialize, Serialize};
use starknet::core::types::contract::legacy::LegacyContractClass;
use starknet::core::types::FieldElement;

use crate::hive_utils::kakarot::compute_starknet_address;
use crate::hive_utils::madara_utils::{
    genesis_approve_kakarot, genesis_fund_starknet_address, genesis_set_bytecode,
    genesis_set_storage_kakarot_contract_account, genesis_set_storage_starknet_contract,
};
use crate::hive_utils::types::{ClassHash, ContractAddress, ContractStorageKey, Felt, StorageValue};

// Replicated from tests/test_utils/macros.rs
#[macro_export]
macro_rules! root_project_path {
    ($relative_path:expr) => {{
        find_project_root_path(None).expect("Failed to find project root").join(Path::new(&$relative_path))
    }};
}

#[derive(Deserialize, Serialize)]
pub struct GenesisLoader {
    pub madara_path: Option<String>,
    pub contract_classes: Vec<(ClassHash, ContractClassPath)>,
    pub contracts: Vec<(ContractAddress, ClassHash)>,
    pub storage: Vec<(ContractStorageKey, StorageValue)>,
    pub fee_token_address: ContractAddress,
    pub seq_addr_updated: bool,
}

#[derive(Deserialize, Serialize)]
pub struct ContractClassPath {
    pub path: String,
    pub version: u8,
}

/// Types from https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HiveGenesisConfig {
    pub config: Config,
    pub coinbase: Address,
    pub difficulty: U64,
    pub extra_data: Bytes,
    pub gas_limit: U64,
    pub nonce: U64,
    pub timestamp: U64,
    pub alloc: HashMap<Address, AccountInfo>,
}

impl HiveGenesisConfig {
    pub fn from_file(path: &str) -> Result<Self> {
        let hive_genesis_file = File::open(path)?;
        Ok(serde_json::from_reader(hive_genesis_file)?)
    }
}

// Define constant addresses for Kakarot contracts
lazy_static! {
    pub static ref KAKAROT_ADDRESS: FieldElement = FieldElement::from_hex_be("0x9001").unwrap(); // Safe unwrap, 0x9001
    pub static ref DEPLOYER_ACCOUNT_ADDRESS: FieldElement = FieldElement::from_hex_be("0x9003").unwrap(); // Safe unwrap, 0x9003
}

fn kakarot_contracts_class_hashes() -> Vec<(String, FieldElement)> {
    dotenv::dotenv().ok();
    let compiled_kakarot_path = root_project_path!(std::env::var("COMPILED_KAKAROT_PATH").expect(
        "Expected a COMPILED_KAKAROT_PATH environment variable, set up your .env file or use \
         `./scripts/make_with_env.sh test`"
    ));

    let paths = walkdir::WalkDir::new(compiled_kakarot_path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|f| f.file_type().is_file() && f.path().extension().is_some_and(|ext| ext == "json"))
        .map(|e| e.into_path());

    // Deserialize each contract file into a `LegacyContractClass` object.
    // Compute the class hash of each contract.
    paths
        .map(|path| {
            let contract_class =
                fs::read_to_string(path.clone()).unwrap_or_else(|_| panic!("Failed to read file: {}", path.display()));
            let contract_class: LegacyContractClass = serde_json::from_str(&contract_class)
                .unwrap_or_else(|_| panic!("Failed to deserialize contract from file: {}", path.display()));

            let filename = path
                .file_stem()
                .expect("File has no stem")
                .to_str()
                .expect("Cannot convert filename to string")
                .to_owned();

            // Compute the class hash
            (filename, contract_class.class_hash().expect("Failed to compute class hash"))
        })
        .collect()
}

/// Convert Hive Genesis Config to Madara Genesis Config
///
/// This function will:
/// 1. Load the Madara genesis file
/// 2. Compute the class hash of Kakarot contracts
/// 3. Add Kakarot contracts to Loader
/// 4. Add Hive accounts to Loader (fund, storage, bytecode, proxy implementation)
/// 5. Serialize Loader to Madara genesis file
pub async fn serialize_hive_to_madara_genesis_config(
    hive_genesis: HiveGenesisConfig,
    mut madara_loader: GenesisLoader,
    combined_genesis_path: &Path,
    compiled_path: &Path,
) -> Result<(), IoError> {
    // Load the class hashes
    let class_hashes = kakarot_contracts_class_hashes();

    // { contract : class_hash }
    let mut kakarot_contracts = HashMap::<String, FieldElement>::new();

    // Add Kakarot contracts Contract Classes to loader
    // Vec so no need to sort
    class_hashes.iter().for_each(|(filename, class_hash)| {
        madara_loader.contract_classes.push((
            Felt(*class_hash),
            ContractClassPath {
                // Add the compiled path to the Kakarot contract filename
                path: compiled_path.join(filename).with_extension("json").into_os_string().into_string().unwrap(), /* safe unwrap,
                                                                                             * valid path */
                version: 0,
            },
        ));

        // Add Kakarot contracts {contract : class_hash} to Kakarot Contracts HashMap
        // Remove .json from filename to get contract name
        kakarot_contracts.insert(filename.to_string(), *class_hash);
    });

    // Set the Kakarot contracts address and proxy class hash
    let account_proxy_class_hash = *kakarot_contracts.get("proxy").expect("Failed to get proxy class hash");
    let contract_account_class_hash =
        *kakarot_contracts.get("contract_account").expect("Failed to get contract_account class hash");
    let eoa_class_hash = *kakarot_contracts.get("externally_owned_account").expect("Failed to get eoa class hash");

    // Add Kakarot contracts to Loader
    madara_loader.contracts.push((
        Felt(*KAKAROT_ADDRESS),
        Felt(*kakarot_contracts.get("kakarot").expect("Failed to get kakarot class hash")),
    ));
    madara_loader.contracts.push((
        Felt(*DEPLOYER_ACCOUNT_ADDRESS),
        Felt(*kakarot_contracts.get("OpenzeppelinAccount").expect("Failed to get deployer account class hash")),
    ));

    // Set storage keys of Kakarot contract
    // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/constants.cairo
    let storage_keys = [
        ("native_token_address", FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap()),
        ("contract_account_class_hash", contract_account_class_hash),
        ("externally_owned_account", eoa_class_hash),
        ("account_proxy_class_hash", account_proxy_class_hash),
        // TODO: Use DEPLOY_FEE constant https://github.com/kkrt-labs/kakarot-rpc/pull/431/files#diff-88f745498d0aaf0b185085d99a74f0feaf253f047babc85770847931e7f726c3R125
        ("deploy_fee", FieldElement::from(100000_u64)),
    ];

    storage_keys.into_iter().for_each(|(key, value)| {
        let storage = genesis_set_storage_starknet_contract(*KAKAROT_ADDRESS, key, &[], value, 0);
        madara_loader.storage.push(storage);
    });

    // Add Hive accounts to loader
    // Convert the EVM accounts to Starknet accounts using compute_starknet_address
    // Sort by key to ensure deterministic order
    let mut hive_accounts: Vec<(reth_primitives::H160, AccountInfo)> = hive_genesis.alloc.into_iter().collect();
    hive_accounts.sort_by_key(|(address, _)| *address);
    hive_accounts.into_iter().for_each(|(evm_address, account_info)| {
        // Use the given Kakarot contract address and declared proxy class hash for compute_starknet_address
        let starknet_address = compute_starknet_address(
            *KAKAROT_ADDRESS,
            account_proxy_class_hash,
            FieldElement::from_byte_slice_be(evm_address.as_bytes()).unwrap(), /* safe unwrap since evm_address
                                                                                * is 20 bytes */
        );
        // Push to contracts
        madara_loader.contracts.push((Felt(starknet_address), Felt(account_proxy_class_hash)));

        // Set the balance of the account and approve Kakarot for infinite allowance
        // Call genesis_fund_starknet_address util to get the storage tuples
        let balances = genesis_fund_starknet_address(starknet_address, account_info.balance);
        let allowance = genesis_approve_kakarot(starknet_address, *KAKAROT_ADDRESS, U256::MAX);
        balances.into_iter().zip(allowance.into_iter()).for_each(|(balance, allowance)| {
            madara_loader.storage.push(balance);
            madara_loader.storage.push(allowance);
        });

        // Set the storage of the account, if any
        if let Some(storage) = account_info.storage {
            let mut storage: Vec<(U256, U256)> = storage.into_iter().collect();
            storage.sort_by_key(|(key, _)| *key);
            storage.into_iter().for_each(|(key, value)| {
                // Call genesis_set_storage_kakarot_contract_account util to get the storage tuples
                let storages = genesis_set_storage_kakarot_contract_account(starknet_address, key, value);
                storages.into_iter().for_each(|storage| {
                    madara_loader.storage.push(storage);
                });
            });
        }

        // Determine the proxy implementation class hash based on whether bytecode is present
        // Set the bytecode to the storage of the account, if any
        let proxy_implementation_class_hash = if let Some(bytecode) = account_info.code {
            let bytecode_len =
                genesis_set_storage_starknet_contract(starknet_address, "bytecode_len_", &[], bytecode.len().into(), 0);
            let bytecode = genesis_set_bytecode(&bytecode, starknet_address);

            // Set the bytecode of the account
            madara_loader.storage.extend(bytecode);
            // Set the bytecode length of the account
            madara_loader.storage.push(bytecode_len);

            // Set the Owner
            let owner =
                genesis_set_storage_starknet_contract(starknet_address, "Ownable_owner", &[], *KAKAROT_ADDRESS, 0);
            madara_loader.storage.push(owner);

            // Since it has bytecode, it's a contract account
            contract_account_class_hash
        } else {
            // Set kakarot address
            let kakarot_address =
                genesis_set_storage_starknet_contract(starknet_address, "kakarot_address", &[], *KAKAROT_ADDRESS, 0);
            madara_loader.storage.push(kakarot_address);

            // Since it has no bytecode, it's an externally owned account
            eoa_class_hash
        };

        // Set the proxy implementation of the account to the determined class hash
        let proxy_implementation_storage = genesis_set_storage_starknet_contract(
            starknet_address,
            "_implementation",
            &[],
            proxy_implementation_class_hash,
            0, // 0 since it's storage value is felt
        );
        madara_loader.storage.push(proxy_implementation_storage);

        // Set the evm address of the account and the "is_initialized" flag
        let evm_address: Felt252Wrapper = evm_address.into();
        let evm_address =
            genesis_set_storage_starknet_contract(starknet_address, "evm_address", &[], evm_address.into(), 0);
        let is_initialized =
            genesis_set_storage_starknet_contract(starknet_address, "is_initialized_", &[], FieldElement::ONE, 0);
        madara_loader.storage.push(evm_address);
        madara_loader.storage.push(is_initialized);
    });

    let combined_genesis_file = File::options()
        .create(true)
        .write(true)
        .append(false)
        .open(combined_genesis_path)
        .unwrap_or_else(|_| panic!("Failed to open file at path {:?}", combined_genesis_path));
    // Serialize the loader to a file
    serde_json::to_writer_pretty(combined_genesis_file, &madara_loader)?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chain_id: i128,
    pub homestead_block: i128,
    pub eip150_block: i128,
    pub eip150_hash: H256,
    pub eip155_block: i128,
    pub eip158_block: i128,
}

#[derive(Serialize, Deserialize)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<U256, U256>>,
}
