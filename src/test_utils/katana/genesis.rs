use crate::{
    providers::eth_provider::utils::split_u256,
    test_utils::constants::{
        ACCOUNT_AUTHORIZED_MESSAGE_HASHES, ACCOUNT_CAIRO1_HELPERS_CLASS_HASH, ACCOUNT_EVM_ADDRESS,
        ACCOUNT_IMPLEMENTATION, EIP_155_AUTHORIZED_MESSAGE_HASHES, KAKAROT_ACCOUNT_CONTRACT_CLASS_HASH,
        KAKAROT_BASE_FEE, KAKAROT_BLOCK_GAS_LIMIT, KAKAROT_CAIRO1_HELPERS_CLASS_HASH, KAKAROT_CHAIN_ID,
        KAKAROT_COINBASE, KAKAROT_EVM_TO_STARKNET_ADDRESS, KAKAROT_NATIVE_TOKEN_ADDRESS, KAKAROT_PREV_RANDAO,
        KAKAROT_UNINITIALIZED_ACCOUNT_CLASS_HASH, OWNABLE_OWNER,
    },
};
use alloy_primitives::{B256, U256};
use alloy_signer_local::PrivateKeySigner;
use eyre::{eyre, OptionExt, Result};
use katana_primitives::{
    contract::{ContractAddress, StorageKey, StorageValue},
    genesis::{
        allocation::DevAllocationsGenerator,
        constant::{DEFAULT_FEE_TOKEN_ADDRESS, DEFAULT_PREFUNDED_ACCOUNT_BALANCE},
        json::{
            ClassNameOrHash, FeeTokenConfigJson, GenesisAccountJson, GenesisClassJson, GenesisContractJson,
            GenesisJson, PathOrFullArtifact,
        },
    },
};
use rayon::prelude::*;
use serde::Serialize;
use serde_json::Value;
use serde_with::serde_as;
use starknet::core::{
    serde::unsigned_field_element::UfeHex,
    types::{
        contract::{legacy::LegacyContractClass, SierraClass},
        Felt,
    },
    utils::{get_contract_address, get_storage_var_address, get_udc_deployed_address, UdcUniqueness},
};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    marker::PhantomData,
    path::PathBuf,
    str::FromStr,
    sync::LazyLock,
};
use walkdir::WalkDir;

pub static SALT: LazyLock<Felt> = LazyLock::new(|| Felt::from_bytes_be(&[0u8; 32]));

#[serde_as]
#[derive(Serialize, Debug)]
pub struct Hex(#[serde_as(as = "UfeHex")] pub Felt);

#[derive(Serialize, Debug)]
pub struct KatanaManifest {
    pub declarations: HashMap<String, Hex>,
    pub deployments: HashMap<String, Hex>,
}

#[derive(Debug, Clone, Default)]
pub struct Uninitialized;
#[derive(Debug, Clone)]
pub struct Loaded;
#[derive(Debug, Clone)]
pub struct Initialized;

#[derive(Debug, Clone, Default)]
pub struct KatanaGenesisBuilder<T = Uninitialized> {
    coinbase: Felt,
    classes: Vec<GenesisClassJson>,
    class_hashes: HashMap<String, Felt>,
    contracts: BTreeMap<ContractAddress, GenesisContractJson>,
    accounts: BTreeMap<ContractAddress, GenesisAccountJson>,
    fee_token_storage: BTreeMap<StorageKey, StorageValue>,
    cache: HashMap<String, Felt>,
    status: PhantomData<T>,
}

// Copy pasted from Dojo repository as it is part of the Katana binary
// https://github.com/dojoengine/dojo/blob/main/bin/katana/src/utils.rs#L6
fn parse_seed(seed: &str) -> [u8; 32] {
    let seed = seed.as_bytes();

    if seed.len() >= 32 {
        unsafe { *seed[..32].as_ptr().cast::<[u8; 32]>() }
    } else {
        let mut actual_seed = [0u8; 32];
        seed.iter().enumerate().for_each(|(i, b)| actual_seed[i] = *b);
        actual_seed
    }
}

impl<T> KatanaGenesisBuilder<T> {
    pub fn update_state<State>(self) -> KatanaGenesisBuilder<State> {
        KatanaGenesisBuilder {
            coinbase: self.coinbase,
            classes: self.classes,
            class_hashes: self.class_hashes,
            contracts: self.contracts,
            accounts: self.accounts,
            fee_token_storage: self.fee_token_storage,
            cache: self.cache,
            status: PhantomData::<State>,
        }
    }

    #[must_use]
    pub fn with_dev_allocation(mut self, amount: u16) -> Self {
        let dev_allocations = DevAllocationsGenerator::new(amount)
            .with_balance(U256::from(DEFAULT_PREFUNDED_ACCOUNT_BALANCE))
            .with_seed(parse_seed("0"))
            .generate()
            .into_iter()
            .map(|(address, account)| {
                (
                    address,
                    GenesisAccountJson {
                        public_key: account.public_key,
                        private_key: Some(account.private_key),
                        balance: account.balance,
                        nonce: account.nonce,
                        class: None,
                        storage: account.storage.clone(),
                    },
                )
            });
        self.accounts.extend(dev_allocations);
        self
    }

    fn kakarot_class_hash(&self) -> Result<Felt> {
        self.class_hashes.get("kakarot").copied().ok_or_eyre("Missing Kakarot class hash")
    }

    pub fn account_contract_class_hash(&self) -> Result<Felt> {
        self.class_hashes.get("account_contract").copied().ok_or_eyre("Missing account contract class hash")
    }

    pub fn uninitialized_account_class_hash(&self) -> Result<Felt> {
        self.class_hashes.get("uninitialized_account").copied().ok_or_eyre("Missing uninitialized account class hash")
    }

    pub fn cairo1_helpers_class_hash(&self) -> Result<Felt> {
        self.class_hashes.get("cairo1_helpers").copied().ok_or_eyre("Missing cairo1 helpers class hash")
    }
}

impl KatanaGenesisBuilder<Uninitialized> {
    /// Load the classes from the given path. Computes the class hashes and stores them in the builder.
    pub fn load_classes(mut self, path: PathBuf) -> KatanaGenesisBuilder<Loaded> {
        let entries = WalkDir::new(path).into_iter().filter(|e| e.is_ok() && e.as_ref().unwrap().file_type().is_file());
        let classes = entries
            .par_bridge()
            .filter_map(|entry| {
                let path = entry.unwrap().path().to_path_buf();
                // Skip class_hashes.json file
                if path.file_name().map_or(false, |name| name == "class_hashes.json") {
                    return None;
                }

                let artifact = fs::read_to_string(&path).expect("Failed to read artifact");
                let artifact = serde_json::from_str(&artifact).expect("Failed to parse artifact");
                let class_hash = compute_class_hash(&artifact)
                    .inspect_err(|e| eprintln!("Failed to compute class hash: {e:?} for {path:?}"))
                    .ok()?;
                Some((
                    path,
                    GenesisClassJson {
                        class: PathOrFullArtifact::Artifact(artifact),
                        class_hash: Some(class_hash),
                        name: None,
                    },
                ))
            })
            .collect::<Vec<_>>();

        self.class_hashes = classes
            .iter()
            .map(|(path, class)| {
                (
                    path.file_stem().unwrap().to_str().unwrap().to_string(),
                    class.class_hash.expect("all class hashes should be computed"),
                )
            })
            .collect();
        self.classes = classes.into_iter().map(|(_, class)| class).collect();

        self.update_state()
    }
}

impl KatanaGenesisBuilder<Loaded> {
    /// Add the Kakarot contract to the genesis. Updates the state to [Initialized].
    /// Once in the [Initialized] status, the builder can be built.
    pub fn with_kakarot(mut self, coinbase_address: Felt, chain_id: Felt) -> Result<KatanaGenesisBuilder<Initialized>> {
        let kakarot_class_hash = self.kakarot_class_hash()?;

        let account_contract_class_hash = self.account_contract_class_hash()?;
        let uninitialized_account_class_hash = self.uninitialized_account_class_hash()?;
        let cairo1_helpers_class_hash = self.cairo1_helpers_class_hash()?;
        let block_gas_limit = 20_000_000u64.into();

        // Construct the kakarot contract address. Based on the constructor args from
        // https://github.com/kkrt-labs/kakarot/blob/main/src/kakarot/kakarot.cairo#L23
        let kakarot_address = ContractAddress::new(get_udc_deployed_address(
            *SALT,
            kakarot_class_hash,
            &UdcUniqueness::NotUnique,
            &[
                Felt::ZERO,
                DEFAULT_FEE_TOKEN_ADDRESS.0,
                account_contract_class_hash,
                uninitialized_account_class_hash,
                cairo1_helpers_class_hash,
                block_gas_limit,
                chain_id,
            ],
        ));
        // Cache the address for later use.
        self.cache.insert("kakarot_address".to_string(), kakarot_address.0);
        self.cache.insert("cairo1_helpers".to_string(), cairo1_helpers_class_hash);

        // Construct the kakarot contract storage.
        let kakarot_storage = [
            (storage_addr(KAKAROT_NATIVE_TOKEN_ADDRESS)?, *DEFAULT_FEE_TOKEN_ADDRESS),
            (storage_addr(KAKAROT_ACCOUNT_CONTRACT_CLASS_HASH)?, account_contract_class_hash),
            (storage_addr(KAKAROT_UNINITIALIZED_ACCOUNT_CLASS_HASH)?, uninitialized_account_class_hash),
            (storage_addr(KAKAROT_CAIRO1_HELPERS_CLASS_HASH)?, cairo1_helpers_class_hash),
            (storage_addr(KAKAROT_COINBASE)?, coinbase_address),
            (storage_addr(KAKAROT_BASE_FEE)?, Felt::ZERO),
            (storage_addr(KAKAROT_PREV_RANDAO)?, Felt::ZERO),
            (storage_addr(KAKAROT_BLOCK_GAS_LIMIT)?, block_gas_limit),
            (storage_addr(KAKAROT_CHAIN_ID)?, chain_id),
        ]
        .into_iter()
        .collect();

        let kakarot = GenesisContractJson {
            class: Some(ClassNameOrHash::Hash(kakarot_class_hash)),
            balance: None,
            nonce: None,
            storage: Some(kakarot_storage),
        };

        self.contracts.insert(kakarot_address, kakarot);
        self.coinbase = coinbase_address;

        Ok(self.update_state())
    }
}

impl KatanaGenesisBuilder<Initialized> {
    /// Add an EOA to the genesis. The EOA is deployed to the address derived from the given private key.
    pub fn with_eoa(mut self, private_key: B256) -> Result<Self> {
        let evm_address = Self::evm_address(private_key)?;

        let kakarot_address = self.cache_load("kakarot_address")?;
        let account_contract_class_hash = self.account_contract_class_hash()?;
        let cairo1_helpers_class_hash = self.cairo1_helpers_class_hash()?;

        // Set the eoa storage
        let mut eoa_storage: BTreeMap<StorageKey, Felt> = [
            (storage_addr(ACCOUNT_EVM_ADDRESS)?, evm_address),
            (storage_addr(OWNABLE_OWNER)?, kakarot_address),
            (storage_addr(ACCOUNT_IMPLEMENTATION)?, account_contract_class_hash),
            (storage_addr(ACCOUNT_CAIRO1_HELPERS_CLASS_HASH)?, cairo1_helpers_class_hash),
        ]
        .into_iter()
        .collect();

        for hash in EIP_155_AUTHORIZED_MESSAGE_HASHES {
            let h = U256::from_str(hash).expect("Failed to parse EIP 155 authorized message hash");
            let [low, high] = split_u256::<Felt>(h);
            eoa_storage.insert(get_storage_var_address(ACCOUNT_AUTHORIZED_MESSAGE_HASHES, &[low, high])?, Felt::ONE);
        }

        let eoa = GenesisContractJson {
            class: Some(ClassNameOrHash::Hash(account_contract_class_hash)),
            balance: None,
            nonce: None,
            storage: Some(eoa_storage),
        };

        let starknet_address = self.compute_starknet_address(evm_address)?;
        self.contracts.insert(starknet_address, eoa);

        // Set the allowance for the EOA to the Kakarot contract.
        let key = get_storage_var_address("ERC20_allowances", &[*starknet_address, kakarot_address])?;
        let storage = [(key, u128::MAX.into()), (key + Felt::ONE, u128::MAX.into())].into_iter();
        self.fee_token_storage.extend(storage);

        // Write the address to the Kakarot evm to starknet mapping
        let kakarot_address = ContractAddress::new(kakarot_address);
        let kakarot_contract = self.contracts.get_mut(&kakarot_address).ok_or_eyre("Kakarot contract missing")?;
        kakarot_contract
            .storage
            .get_or_insert_with(BTreeMap::new)
            .extend([(get_storage_var_address(KAKAROT_EVM_TO_STARKNET_ADDRESS, &[evm_address])?, starknet_address.0)]);

        Ok(self)
    }

    /// Fund the starknet address deployed for the evm address of the passed private key
    /// with the given amount of tokens.
    pub fn fund(mut self, pk: B256, amount: U256) -> Result<Self> {
        let evm_address = Self::evm_address(pk)?;
        let starknet_address = self.compute_starknet_address(evm_address)?;
        let eoa = self.contracts.get_mut(&starknet_address).ok_or_eyre("Missing EOA contract")?;

        let key = get_storage_var_address("ERC20_balances", &[*starknet_address])?;
        let amount_split = split_u256::<u128>(amount);

        let storage = [(key, amount_split[0].into()), (key + Felt::ONE, amount_split[1].into())].into_iter();
        self.fee_token_storage.extend(storage);

        eoa.balance = Some(amount);

        Ok(self)
    }

    /// Consume the [`KatanaGenesisBuilder`] and returns the corresponding [`GenesisJson`].
    pub fn build(self) -> Result<GenesisJson> {
        Ok(GenesisJson {
            sequencer_address: self.compute_starknet_address(self.coinbase)?,
            classes: self.classes,
            fee_token: FeeTokenConfigJson {
                name: "Ether".to_string(),
                symbol: "ETH".to_string(),
                decimals: 18,
                storage: Some(self.fee_token_storage),
                ..Default::default()
            },
            accounts: self.accounts,
            contracts: self.contracts,
            ..Default::default()
        })
    }

    /// Returns the manifest of the genesis.
    pub fn manifest(&self) -> KatanaManifest {
        KatanaManifest {
            declarations: self.class_hashes().clone().into_iter().map(|(k, v)| (k, Hex(v))).collect(),
            deployments: self.cache().clone().into_iter().map(|(k, v)| (k, Hex(v))).collect(),
        }
    }

    /// Compute the Starknet address for the given Ethereum address.
    pub fn compute_starknet_address(&self, evm_address: Felt) -> Result<ContractAddress> {
        let kakarot_address = self.cache_load("kakarot_address")?;
        let uninitialized_account_class_hash = self.uninitialized_account_class_hash()?;

        Ok(ContractAddress::new(get_contract_address(
            evm_address,
            uninitialized_account_class_hash,
            &[Felt::ONE, evm_address],
            kakarot_address,
        )))
    }

    fn evm_address(pk: B256) -> Result<Felt> {
        Ok(Felt::from_bytes_be_slice(&PrivateKeySigner::from_bytes(&pk)?.address().into_array()))
    }

    pub fn cache_load(&self, key: &str) -> Result<Felt> {
        self.cache.get(key).copied().ok_or_else(|| eyre!("Cache miss for {key} address"))
    }

    pub const fn cache(&self) -> &HashMap<String, Felt> {
        &self.cache
    }

    pub const fn class_hashes(&self) -> &HashMap<String, Felt> {
        &self.class_hashes
    }
}

fn compute_class_hash(class: &Value) -> Result<Felt> {
    if let Ok(sierra) = serde_json::from_value::<SierraClass>(class.clone()) {
        Ok(sierra.class_hash()?)
    } else {
        let casm: LegacyContractClass = serde_json::from_value(class.clone())?;
        Ok(casm.class_hash()?)
    }
}

fn storage_addr(var_name: &str) -> Result<Felt> {
    Ok(get_storage_var_address(var_name, &[])?)
}
