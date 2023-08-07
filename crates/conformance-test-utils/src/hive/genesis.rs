use std::collections::HashMap;
use std::fs;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use eyre::Result;
use kakarot_rpc_core::test_utils::deploy_helpers::compute_kakarot_contracts_class_hash;
use pallet_starknet::genesis_loader::{read_file_to_string, ContractClass, GenesisLoader, HexFelt};
use reth_primitives::{Address, Bytes, H256, U256, U64};
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

use crate::kakarot::compute_starknet_address;
use crate::madara::utils::{
    genesis_fund_starknet_address, genesis_set_bytecode, genesis_set_storage_kakarot_contract_account,
};
use crate::types::Felt;

/// Types from https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58
#[derive(Serialize, Deserialize, Debug)]
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
    pub fn new() -> Result<Self, serde_json::Error> {
        serde_json::from_str(std::include_str!("./genesis.json"))
    }
}

/// Convert Hive Genesis Config to Madara Genesis Config
///
/// This function will:
/// 1. Load the Madara genesis file
/// 2. Construct a Starknet test sequencer
/// 3. Declare Kakarot contracts
/// 4. Add Kakarot contracts to Loader
/// 5. Add Hive accounts to Loader
/// 6. Serialize Loader to Madara genesis file
pub async fn serialize_hive_to_madara_genesis_config(hive_genesis: HiveGenesisConfig) -> Result<(), IoError> {
    // Load the Madara genesis file
    let mut loader: GenesisLoader =
        serde_json::from_str(&read_file_to_string("crates/conformance-test-utils/src/madara/genesis.json"))
            .expect("Failed to load Madara genesis file");

    // Get compiled path of contracts on loader
    let mut compiled_path = PathBuf::from("");
    if let ContractClass::Path { path, .. } = &loader.contract_classes[0].1 {
        compiled_path = PathBuf::from(path);

        // Remove filename
        compiled_path.pop();
    }

    // Declare Kakarot contracts
    let class_hashes = compute_kakarot_contracts_class_hash();

    // { contract : (address, class_hash) }
    let mut kakarot_contracts = HashMap::<String, (FieldElement, FieldElement)>::new();
    // Add Kakarot contracts Contract Classes to loader
    // Vec so no need to sort
    class_hashes.iter().enumerate().for_each(|(index, (filename, class_hash))| {
        loader.contract_classes.push((
            HexFelt(*class_hash),
            ContractClass::Path {
                // Add the compiled path to the Kakarot contract filename
                path: compiled_path.join(filename).with_extension("json").into_os_string().into_string().unwrap(), /* safe unwrap,
                                                                                             * valid path */
                version: 0,
            },
        ));
        // Add Kakarot contracts (address, class_hash) to Kakarot Contracts HashMap
        // Remove .json from filename to get contract name
        // 36865 is over 0x9000
        kakarot_contracts.insert(filename.replace(".json", ""), (FieldElement::from(36865 + index), *class_hash));
    });

    // Get Kakarot contracts address and proxy class hash
    let kakarot_address = kakarot_contracts.get("kakarot").unwrap().0;
    let account_proxy_class_hash = kakarot_contracts.get("proxy").unwrap().1;

    // Add Kakarot contracts to Loader
    // Convert the HashMap to Vec and sort by key to ensure deterministic order
    let mut kakarot_contracts: Vec<(String, (FieldElement, FieldElement))> = kakarot_contracts.into_iter().collect();
    kakarot_contracts.sort_by_key(|(name, (_, _))| name.clone());
    kakarot_contracts.iter().for_each(|(_, (address, class_hash))| {
        loader.contracts.push((HexFelt(*address), HexFelt(*class_hash)));
    });

    // Add Hive accounts to loader
    // Convert the EVM accounts to Starknet accounts using compute_starknet_address
    // Sort by key to ensure deterministic order
    let mut hive_accounts: Vec<(reth_primitives::H160, AccountInfo)> = hive_genesis.alloc.into_iter().collect();
    hive_accounts.sort_by_key(|(address, _)| *address);
    hive_accounts.iter().for_each(|(evm_address, account_info)| {
        // Use the given Kakarot contract address and declared proxy class hash for compute_starknet_address
        let starknet_address = compute_starknet_address(
            kakarot_address,
            account_proxy_class_hash,
            FieldElement::from_byte_slice_be(evm_address.as_bytes()).unwrap(), /* safe unwrap since evm_address
                                                                                * is 20 bytes */
        );
        // Push to contracts
        loader.contracts.push((HexFelt(starknet_address), HexFelt(account_proxy_class_hash)));

        // Set the balance of the account
        // Call genesis_fund_starknet_address util to get the storage tuples
        let balance_storage_tuples = genesis_fund_starknet_address(starknet_address, account_info.balance);
        balance_storage_tuples.iter().for_each(|balance_storage_tuple| {
            loader.storage.push(unsafe {
                std::mem::transmute::<((Felt, Felt), Felt), ((HexFelt, HexFelt), HexFelt)>(*balance_storage_tuple)
            });
        });

        // Set the storage of the account, if any
        if let Some(storage) = account_info.storage.as_ref() {
            let mut storage: Vec<(U256, U256)> = storage.iter().map(|(k, v)| (*k, *v)).collect();
            storage.sort_by_key(|(key, _)| *key);
            storage.iter().for_each(|(key, value)| {
                // Call genesis_set_storage_kakarot_contract_account util to get the storage tuples
                let storage_tuples = genesis_set_storage_kakarot_contract_account(starknet_address, *key, *value);
                storage_tuples.iter().for_each(|storage_tuples| {
                    loader.storage.push(unsafe {
                        std::mem::transmute::<((Felt, Felt), Felt), ((HexFelt, HexFelt), HexFelt)>(*storage_tuples)
                    });
                });
            });
        }

        // Set the bytecode of the account, if any
        if let Some(bytecode) = account_info.code.as_ref() {
            // Call genesis_set_code_kakarot_contract_account util to get the storage tuples
            let code_storage_tuples = genesis_set_bytecode(bytecode, starknet_address);
            code_storage_tuples.iter().for_each(|code_storage_tuple| {
                loader.storage.push(unsafe {
                    std::mem::transmute::<((Felt, Felt), Felt), ((HexFelt, HexFelt), HexFelt)>(*code_storage_tuple)
                });
            });
        }
    });

    // Serialize the loader to a string
    let madara_genesis_str = serde_json::to_string_pretty(&loader)?;
    // Write the string to a file
    fs::write(Path::new("src/hive/madara_genesis.json"), madara_genesis_str)?;

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chain_id: i128,
    pub homestead_block: i128,
    pub eip150_block: i128,
    pub eip150_hash: H256,
    pub eip155_block: i128,
    pub eip158_block: i128,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<U256, U256>>,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use pallet_starknet::genesis_loader::GenesisLoader;
    use reth_primitives::U256;

    use super::*;

    #[test]
    fn test_read_hive_genesis() {
        // Read the hive genesis file
        let genesis = HiveGenesisConfig::new().expect("Failed to read genesis.json");

        // Verify the genesis file has the expected number of accounts
        assert_eq!(genesis.alloc.len(), 7);

        // Verify balance of each account is not empty
        assert!(genesis.alloc.values().all(|account_info| account_info.balance >= U256::from(0)));

        // Verify the storage field for each account
        // Since there is only one account with non-empty storage, we can hardcode the expected values
        assert!(genesis.alloc.values().all(|account_info| {
            account_info.storage.as_ref().map_or(true, |storage| {
                storage.len() == 2
                    && *storage
                        .get(
                            &U256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
                                .unwrap(),
                        )
                        .unwrap()
                        == U256::from_str("0x1234").unwrap()
                    && *storage
                        .get(
                            &U256::from_str("0x6661e9d6d8b923d5bbaab1b96e1dd51ff6ea2a93520fdc9eb75d059238b8c5e9")
                                .unwrap(),
                        )
                        .unwrap()
                        == U256::from_str("0x01").unwrap()
            })
        }));

        // Verify the code field for each account, if exists, is not empty
        assert!(
            genesis.alloc.values().all(|account_info| account_info.code.as_ref().map_or(true, |code| !code.is_empty()))
        );
    }

    #[tokio::test]
    async fn test_madara_genesis() {
        let hive_genesis = HiveGenesisConfig::new().expect("Failed to read genesis.json");
        serialize_hive_to_madara_genesis_config(hive_genesis).await.unwrap();
        let loader: GenesisLoader =
            serde_json::from_str(std::include_str!("madara_genesis.json")).expect("Failed to read madara_genesis.json");
        assert_eq!(9 + 5 + 7, loader.contracts.len()); // 9 original + 5 Kakarot contracts + 7 hive
    }
}
