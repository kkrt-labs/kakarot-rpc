use std::collections::HashMap;
use std::fs;
use std::io::Error as IoError;
use std::path::{Path, PathBuf};

use eyre::Result;
use kakarot_rpc_core::client::constants::STARKNET_NATIVE_TOKEN;
use kakarot_rpc_core::test_utils::deploy_helpers::compute_kakarot_contracts_class_hash;
use pallet_starknet::genesis_loader::{ContractClass, GenesisLoader, HexFelt};
use reth_primitives::{Address, Bytes, H256, U256, U64};
use serde::{Deserialize, Serialize};
use starknet::core::types::FieldElement;

use crate::kakarot::compute_starknet_address;
use crate::madara::utils::{
    genesis_fund_starknet_address, genesis_set_bytecode, genesis_set_storage_kakarot_contract_account,
    genesis_set_storage_starknet_contract,
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

// Define constant addresses for Kakarot contracts
const KAKAROT_ADDRESSES: &str = "0x9001";
const BLOCKHASH_REGISTRY_ADDRESS: &str = "0x9002";

/// Convert Hive Genesis Config to Madara Genesis Config
///
/// This function will:
/// 1. Load the Madara genesis file
/// 2. Compute the class hash of Kakarot contracts
/// 3. Add Kakarot contracts to Loader
/// 4. Add Hive accounts to Loader (fund, storage, bytecode)
/// 5. Serialize Loader to Madara genesis file
pub async fn serialize_hive_to_madara_genesis_config(
    hive_genesis: HiveGenesisConfig,
    mut madara_loader: GenesisLoader,
    madara_genesis: &Path,
    compiled_path: PathBuf,
) -> Result<(), IoError> {
    // Compute the class hash of Kakarot contracts
    let class_hashes = compute_kakarot_contracts_class_hash();

    // { contract : class_hash }
    let mut kakarot_contracts = HashMap::<String, FieldElement>::new();

    // Add Kakarot contracts Contract Classes to loader
    // Vec so no need to sort
    class_hashes.iter().for_each(|(filename, class_hash)| {
        madara_loader.contract_classes.push((
            HexFelt(*class_hash),
            ContractClass::Path {
                // Add the compiled path to the Kakarot contract filename
                path: compiled_path.join(filename).with_extension("json").into_os_string().into_string().unwrap(), /* safe unwrap,
                                                                                             * valid path */
                version: 0,
            },
        ));

        // Add Kakarot contracts {contract : class_hash} to Kakarot Contracts HashMap
        // Remove .json from filename to get contract name
        kakarot_contracts.insert(filename.replace(".json", ""), *class_hash);
    });

    // Set the Kakarot contracts address and proxy class hash
    let kakarot_address = FieldElement::from_hex_be(KAKAROT_ADDRESSES).unwrap(); // Safe unwrap, 0x9001
    let blockhash_registry_address = FieldElement::from_hex_be(BLOCKHASH_REGISTRY_ADDRESS).unwrap(); // Safe unwrap, 0x9002
    let account_proxy_class_hash = *kakarot_contracts.get("proxy").expect("Failed to get proxy class hash");
    let contract_account_class_hash =
        *kakarot_contracts.get("contract_account").expect("Failed to get contract_account class hash");
    let eoa_class_hash = *kakarot_contracts.get("externally_owned_account").expect("Failed to get eoa class hash");

    // Add Kakarot contracts to Loader
    madara_loader.contracts.push((
        HexFelt(kakarot_address),
        HexFelt(*kakarot_contracts.get("kakarot").expect("Failed to get kakarot class hash")),
    ));
    madara_loader.contracts.push((
        HexFelt(blockhash_registry_address),
        HexFelt(*kakarot_contracts.get("blockhash_registry").expect("Failed to get blockhash_registry class hash")),
    ));

    // Set storage keys of Kakarot contract
    // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/constants.cairo
    let storage_keys = [
        ("native_token_address", FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap()),
        ("contract_account_class_hash", contract_account_class_hash),
        ("externally_owned_account", eoa_class_hash),
        ("account_proxy_class_hash", account_proxy_class_hash),
        ("blockhash_registry_address", blockhash_registry_address), // Safe unwrap 0x9002
    ];

    storage_keys.iter().for_each(|(key, value)| {
        let storage_tuple = genesis_set_storage_starknet_contract(kakarot_address, key, &[], *value, 0);
        madara_loader
            .storage
            .push(unsafe { std::mem::transmute::<((Felt, Felt), Felt), ((HexFelt, HexFelt), HexFelt)>(storage_tuple) });
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
        madara_loader.contracts.push((HexFelt(starknet_address), HexFelt(account_proxy_class_hash)));

        // Set the balance of the account
        // Call genesis_fund_starknet_address util to get the storage tuples
        let balance_storage_tuples = genesis_fund_starknet_address(starknet_address, account_info.balance);
        balance_storage_tuples.iter().for_each(|balance_storage_tuple| {
            madara_loader.storage.push(unsafe {
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
                    madara_loader.storage.push(unsafe {
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
                madara_loader.storage.push(unsafe {
                    std::mem::transmute::<((Felt, Felt), Felt), ((HexFelt, HexFelt), HexFelt)>(*code_storage_tuple)
                });
            });
        }
    });

    // Serialize the loader to a string
    let madara_genesis_str = serde_json::to_string_pretty(&madara_loader)?;
    // Write the string to a file
    fs::write(madara_genesis, madara_genesis_str)?;

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
        // Given
        let hive_genesis = HiveGenesisConfig::new().expect("Failed to read genesis.json");
        let madara_loader = serde_json::from_str::<GenesisLoader>(std::include_str!("../madara/genesis.json")).unwrap();
        let madara_genesis = Path::new("src/hive/madara_genesis.json");
        let compiled_path = PathBuf::from("./cairo-contracts/build");

        // When
        serialize_hive_to_madara_genesis_config(hive_genesis, madara_loader, madara_genesis, compiled_path)
            .await
            .unwrap();

        // Then
        let loader: GenesisLoader =
            serde_json::from_str(include_str!("./madara_genesis.json")).expect("Failed to read madara_genesis.json");
        assert_eq!(9 + 2 + 7, loader.contracts.len()); // 9 original + 2 Kakarot contracts + 7 hive
    }
}
