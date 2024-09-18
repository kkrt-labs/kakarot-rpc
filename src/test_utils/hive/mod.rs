use super::{
    constants::{
        ACCOUNT_CAIRO1_HELPERS_CLASS_HASH, ACCOUNT_IMPLEMENTATION, KAKAROT_EVM_TO_STARKNET_ADDRESS, OWNABLE_OWNER,
    },
    katana::genesis::{KatanaGenesisBuilder, Loaded},
};
use account::{Account, KakarotAccount};
use katana_primitives::{
    contract::ContractAddress,
    genesis::json::{ClassNameOrHash, GenesisContractJson, GenesisJson},
};
use reth_primitives::{Address, Bytes, B256, U256, U64};
use serde::{Deserialize, Serialize};
use starknet::core::{types::Felt, utils::get_storage_var_address};
use std::collections::HashMap;

mod account;

/// Types from <https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58>
#[derive(Serialize, Deserialize, Clone, Debug)]
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

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub chain_id: i128,
    pub homestead_block: i128,
    pub eip150_block: i128,
    pub eip150_hash: Option<B256>,
    pub eip155_block: i128,
    pub eip158_block: i128,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AccountInfo {
    pub balance: U256,
    pub code: Option<Bytes>,
    pub storage: Option<HashMap<U256, U256>>,
}

impl HiveGenesisConfig {
    /// Convert the [`HiveGenesisConfig`] into a [`GenesisJson`] using an [`KatanaGenesisBuilder`]<[Loaded]>. The [Loaded]
    /// marker type indicates that the Kakarot contract classes need to have been loaded into the builder.
    pub fn try_into_genesis_json(self, builder: KatanaGenesisBuilder<Loaded>) -> Result<GenesisJson, eyre::Error> {
        let coinbase_address = Felt::from_bytes_be_slice(self.coinbase.as_slice());
        let builder = builder.with_kakarot(coinbase_address)?;

        // Get the current state of the builder.
        let kakarot_address = builder.cache_load("kakarot_address")?;
        let account_contract_class_hash = builder.account_contract_class_hash()?;

        // Fetch the contracts from the alloc field.
        let mut additional_kakarot_storage = HashMap::with_capacity(self.alloc.len()); // 1 mapping per contract
        let mut fee_token_storage = HashMap::with_capacity(2 * self.alloc.len()); // 2 allowances per contract
        let contracts = self
            .alloc
            .into_iter()
            .map(|(address, info)| {
                let evm_address = Felt::from_bytes_be_slice(address.as_slice());
                let starknet_address = builder.compute_starknet_address(evm_address)?.0;

                // Store the mapping from EVM to Starknet address.
                additional_kakarot_storage.insert(
                    get_storage_var_address(KAKAROT_EVM_TO_STARKNET_ADDRESS, &[evm_address])?,
                    starknet_address,
                );

                // Get the Kakarot account in order to have the account type and storage.
                let code = info.code.unwrap_or_default();
                let storage = info.storage.unwrap_or_default();
                let storage: Vec<(U256, U256)> = storage.into_iter().collect();
                let nonce = if code.is_empty() && storage.is_empty() { U256::ZERO } else { U256::from(1u8) };

                let kakarot_account = KakarotAccount::new(&address, Account { code, storage, nonce, ..Default::default())?;

                let mut kakarot_account_storage: Vec<(Felt, Felt)> =
                    kakarot_account.storage().iter().map(|(k, v)| (*k, *v)).collect();

                // Add the implementation to the storage.
                let implementation_key = get_storage_var_address(ACCOUNT_IMPLEMENTATION, &[])?;
                kakarot_account_storage.append(&mut vec![
                    (implementation_key, account_contract_class_hash),
                    (get_storage_var_address(OWNABLE_OWNER, &[])?, kakarot_address),
                    (
                        get_storage_var_address(ACCOUNT_CAIRO1_HELPERS_CLASS_HASH, &[])?,
                        builder.cache_load("cairo1_helpers")?,
                    ),
                ]);

                let key = get_storage_var_address("ERC20_allowances", &[starknet_address, kakarot_address])?;
                fee_token_storage.insert(key, u128::MAX.into());
                fee_token_storage.insert(key + Felt::ONE, u128::MAX.into());

                Ok((
                    ContractAddress::new(starknet_address),
                    GenesisContractJson {
                        class: Some(ClassNameOrHash::Hash(account_contract_class_hash)),
                        balance: Some(info.balance),
                        nonce: None,
                        storage: Some(kakarot_account_storage.into_iter().collect()),
                    },
                ))
            })
            .collect::<Result<HashMap<_, _>, eyre::Error>>()?;

        // Build the builder
        let kakarot_address = ContractAddress::new(kakarot_address);
        let mut genesis = builder.build()?;

        let kakarot_contract = genesis.contracts.entry(kakarot_address);
        kakarot_contract.and_modify(|contract| {
            contract.storage.get_or_insert_with(HashMap::new).extend(additional_kakarot_storage);
        });

        genesis.fee_token.storage.get_or_insert_with(HashMap::new).extend(fee_token_storage);

        // Add the contracts to the genesis.
        genesis.contracts.extend(contracts);

        Ok(genesis)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        providers::eth_provider::utils::split_u256,
        test_utils::{constants::ACCOUNT_STORAGE, katana::genesis::Initialized},
    };
    use std::{
        path::{Path, PathBuf},
        sync::LazyLock,
    };

    static ROOT: LazyLock<PathBuf> = LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf());
    static HIVE_GENESIS: LazyLock<HiveGenesisConfig> = LazyLock::new(|| {
        let hive_content = std::fs::read_to_string(ROOT.join("src/test_utils/hive/test_data/genesis.json")).unwrap();
        serde_json::from_str(&hive_content).unwrap()
    });
    static GENESIS_BUILDER_LOADED: LazyLock<KatanaGenesisBuilder<Loaded>> =
        LazyLock::new(|| KatanaGenesisBuilder::default().load_classes(ROOT.join("lib/kakarot/build")));
    static GENESIS_BUILDER: LazyLock<KatanaGenesisBuilder<Initialized>> =
        LazyLock::new(|| GENESIS_BUILDER_LOADED.clone().with_kakarot(Felt::ZERO).unwrap());
    static GENESIS: LazyLock<GenesisJson> =
        LazyLock::new(|| HIVE_GENESIS.clone().try_into_genesis_json(GENESIS_BUILDER_LOADED.clone()).unwrap());

    #[test]
    fn test_correct_genesis_len() {
        assert_eq!(GENESIS.contracts.len(), 8);
    }

    #[test]
    fn test_genesis_accounts() {
        for (address, account) in HIVE_GENESIS.alloc.clone() {
            let starknet_address =
                GENESIS_BUILDER.compute_starknet_address(Felt::from_bytes_be_slice(address.as_slice())).unwrap().0;
            let contract = GENESIS.contracts.get(&ContractAddress::new(starknet_address)).unwrap();

            // Check the balance
            assert_eq!(contract.balance, Some(account.balance));
            // Check the storage
            for (key, value) in account.storage.unwrap_or_default() {
                let key = get_storage_var_address(ACCOUNT_STORAGE, &split_u256::<Felt>(key)).unwrap();
                let low =
                    U256::from_be_slice(contract.storage.as_ref().unwrap().get(&key).unwrap().to_bytes_be().as_slice());
                let high = U256::from_be_slice(
                    contract.storage.as_ref().unwrap().get(&(key + Felt::ONE)).unwrap().to_bytes_be().as_slice(),
                );
                let actual_value = low + (high << 128);
                assert_eq!(actual_value, value);
            }
        }
    }
}
