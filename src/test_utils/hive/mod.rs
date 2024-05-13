use std::collections::HashMap;

use ef_testing::evm_sequencer::account::KakarotAccount;
use ethers::types::U256 as EthersU256;
use katana_primitives::{
    contract::ContractAddress,
    genesis::json::{GenesisContractJson, GenesisJson},
};
use reth_primitives::{Address, Bytes, B256, U256, U64};
use serde::{Deserialize, Serialize};
use starknet::core::utils::get_storage_var_address;
use starknet_api::core::ClassHash;
use starknet_crypto::FieldElement;

use super::{
    constants::{
        ACCOUNT_CAIRO1_HELPERS_CLASS_HASH, ACCOUNT_IMPLEMENTATION, ACCOUNT_NONCE, KAKAROT_EVM_TO_STARKNET_ADDRESS,
        OWNABLE_OWNER,
    },
    katana::genesis::{KatanaGenesisBuilder, Loaded},
};

/// Types from https://github.com/ethereum/go-ethereum/blob/master/core/genesis.go#L49C1-L58
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
    /// Convert the [HiveGenesisConfig] into a [GenesisJson] using an [KatanaGenesisBuilder]<[Loaded]>. The [Loaded]
    /// marker type indicates that the Kakarot contract classes need to have been loaded into the builder.
    pub fn try_into_genesis_json(self, builder: KatanaGenesisBuilder<Loaded>) -> Result<GenesisJson, eyre::Error> {
        let coinbase_address = FieldElement::from_byte_slice_be(self.coinbase.as_slice())?;
        let builder = builder.with_kakarot(coinbase_address)?;

        // Get the current state of the builder.
        let kakarot_address = builder.cache_load("kakarot_address")?;
        let account_contract_class_hash = ClassHash(builder.account_contract_class_hash()?.into());

        // Fetch the contracts from the alloc field.
        let mut additional_kakarot_storage = HashMap::with_capacity(self.alloc.len()); // 1 mapping per contract
        let mut fee_token_storage = HashMap::with_capacity(2 * self.alloc.len()); // 2 allowances per contract
        let contracts = self
            .alloc
            .into_iter()
            .map(|(address, info)| {
                let evm_address = FieldElement::from_byte_slice_be(address.as_slice())?;
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
                let kakarot_account = KakarotAccount::new(&address, &code, U256::ZERO, &storage)?;

                let mut kakarot_account_storage: Vec<(FieldElement, FieldElement)> =
                    kakarot_account.storage().iter().map(|(k, v)| ((*k.0.key()).into(), (*v).into())).collect();

                // Add the implementation to the storage.
                let implementation_key = get_storage_var_address(ACCOUNT_IMPLEMENTATION, &[])?;
                kakarot_account_storage.append(&mut vec![
                    (implementation_key, account_contract_class_hash.0.into()),
                    (get_storage_var_address(ACCOUNT_NONCE, &[])?, FieldElement::ONE),
                    (get_storage_var_address(OWNABLE_OWNER, &[])?, kakarot_address),
                    (
                        get_storage_var_address(ACCOUNT_CAIRO1_HELPERS_CLASS_HASH, &[])?,
                        builder.cache_load("cairo1_helpers")?,
                    ),
                ]);

                let key = get_storage_var_address("ERC20_allowances", &[starknet_address, kakarot_address])?;
                fee_token_storage.insert(key, u128::MAX.into());
                fee_token_storage.insert(key + 1u8.into(), u128::MAX.into());

                Ok((
                    ContractAddress::new(starknet_address),
                    GenesisContractJson {
                        class: Some(account_contract_class_hash.0.into()),
                        balance: Some(EthersU256::from_big_endian(&info.balance.to_be_bytes::<32>())),
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
            contract.storage.get_or_insert_with(HashMap::new).extend(additional_kakarot_storage)
        });

        genesis.fee_token.storage.get_or_insert_with(HashMap::new).extend(fee_token_storage);

        // Add the contracts to the genesis.
        genesis.contracts.extend(contracts);

        Ok(genesis)
    }
}

#[cfg(test)]
mod tests {
    use lazy_static::lazy_static;

    use crate::{
        eth_provider::utils::split_u256,
        test_utils::{constants::ACCOUNT_STORAGE, katana::genesis::Initialized},
    };

    use super::*;
    use std::path::{Path, PathBuf};

    lazy_static! {
        static ref ROOT: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        static ref HIVE_GENESIS: HiveGenesisConfig = {
            let hive_content =
                std::fs::read_to_string(ROOT.join("src/test_utils/hive/test_data/genesis.json")).unwrap();
            serde_json::from_str(&hive_content).unwrap()
        };
        static ref GENESIS_BUILDER_LOADED: KatanaGenesisBuilder<Loaded> =
            KatanaGenesisBuilder::default().load_classes(ROOT.join("lib/kakarot/build"));
        static ref GENESIS_BUILDER: KatanaGenesisBuilder<Initialized> =
            GENESIS_BUILDER_LOADED.clone().with_kakarot(FieldElement::ZERO).unwrap();
        static ref GENESIS: GenesisJson =
            HIVE_GENESIS.clone().try_into_genesis_json(GENESIS_BUILDER_LOADED.clone()).unwrap();
    }

    #[test]
    fn test_correct_genesis_len() {
        // Then
        assert_eq!(GENESIS.contracts.len(), 8);
    }

    #[test]
    fn test_genesis_accounts() {
        // Then
        for (address, account) in HIVE_GENESIS.alloc.clone() {
            let starknet_address = GENESIS_BUILDER
                .compute_starknet_address(FieldElement::from_byte_slice_be(address.as_slice()).unwrap())
                .unwrap()
                .0;
            let contract = GENESIS.contracts.get(&ContractAddress::new(starknet_address)).unwrap();

            // Check the balance
            assert_eq!(contract.balance, Some(EthersU256::from_big_endian(&account.balance.to_be_bytes::<32>())));
            // Check the storage
            for (key, value) in account.storage.unwrap_or_default() {
                let key = get_storage_var_address(ACCOUNT_STORAGE, &split_u256::<FieldElement>(key)).unwrap();
                let low =
                    U256::from_be_slice(contract.storage.as_ref().unwrap().get(&key).unwrap().to_bytes_be().as_slice());
                let high = U256::from_be_slice(
                    contract.storage.as_ref().unwrap().get(&(key + 1u8.into())).unwrap().to_bytes_be().as_slice(),
                );
                let actual_value = low + (high << 128);
                assert_eq!(actual_value, value);
            }
        }
    }
}
