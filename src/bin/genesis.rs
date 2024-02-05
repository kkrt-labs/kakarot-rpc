use cairo_lang_starknet::{casm_contract_class::CasmContractClass, contract_class::ContractClass};
use dotenv::dotenv;
use ethers::signers::LocalWallet;
use ethers::signers::Signer;
use ethers::types::U256;
use kakarot_rpc::into_via_wrapper;
use kakarot_rpc::models::felt::Felt252Wrapper;
use katana_primitives::{
    block::{BlockHash, BlockNumber, GasPrices},
    contract::ContractAddress,
    genesis::{
        constant::DEFAULT_FEE_TOKEN_ADDRESS,
        json::{FeeTokenConfigJson, GenesisClassJson, GenesisContractJson, GenesisJson, PathOrFullArtifact},
    },
};
use lazy_static::lazy_static;
use rand::{rngs::SmallRng, RngCore, SeedableRng};
use reth_primitives::Address;
use reth_primitives::B256;
use ruint::aliases::U160;
use serde::Serialize;
use starknet::core::utils::get_contract_address;
use starknet::core::{types::contract::legacy::LegacyContractClass, utils::get_storage_var_address};
use starknet_crypto::FieldElement;
use std::{
    collections::HashMap,
    env::var,
    path::{Path, PathBuf},
    str::FromStr,
};
use walkdir::WalkDir;

lazy_static! {
    static ref GENESIS_FOLDER_PATH: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join(".katana");
    static ref KAKAROT_CONTRACTS_RELATIVE_PATH: PathBuf = GENESIS_FOLDER_PATH.join("../lib/kakarot/build");
    static ref INCORRECT_FELT: String = "INCORRECT".to_string();
    static ref COINBASE_ADDRESS: Address = Address::from(U160::from(0x12345));
}

#[derive(Serialize)]
struct KatanaManifest {
    declarations: HashMap<String, FieldElement>,
    deployments: HashMap<String, ContractAddress>,
}

fn main() {
    // Load the env vars.
    dotenv().ok();

    // Read all the classes.
    let classes = WalkDir::new(KAKAROT_CONTRACTS_RELATIVE_PATH.as_path())
        .into_iter()
        .filter(|e| e.as_ref().unwrap().file_type().is_file())
        .map(|entry| GenesisClassJson {
            class: PathOrFullArtifact::Path(entry.unwrap().path().to_path_buf()),
            class_hash: None,
        })
        .collect::<Vec<_>>();

    // Iter all the classes and compute their hashes.
    let class_hashes = classes
        .iter()
        .map(|class| {
            let path = match &class.class {
                PathOrFullArtifact::Path(path) => path,
                _ => panic!("Expected path"),
            };
            (path.file_stem().unwrap().to_str().unwrap().to_string(), compute_class_hash(path))
        })
        .collect::<HashMap<_, _>>();

    // Set up the Kakarot and EOA contracts.
    let proxy_class_hash = class_hashes.get("proxy").cloned().unwrap();
    let (kakarot_address, kakarot) = set_up_kakarot(&class_hashes);
    let (eoa_address, eoa) = set_up_eoa(&class_hashes, kakarot_address.0, proxy_class_hash);

    // Compute the coinbase address.
    let sequencer_address =
        ContractAddress::new(starknet_address(*COINBASE_ADDRESS, *kakarot_address, proxy_class_hash));

    // Construct the genesis json contracts.
    let contracts = [(kakarot_address, kakarot), (eoa_address, eoa)].iter().cloned().collect::<HashMap<_, _>>();

    // Construct the manifest.
    let manifest = KatanaManifest {
        declarations: class_hashes.clone(),
        deployments: [("kakarot".to_string(), kakarot_address)].into_iter().collect(),
    };

    // Construct the genesis json.
    let genesis = GenesisJson {
        parent_hash: BlockHash::default(),
        state_root: FieldElement::ZERO,
        number: BlockNumber::default(),
        timestamp: 0,
        sequencer_address,
        gas_prices: GasPrices::default(),
        classes,
        fee_token: set_up_fee_token(*eoa_address, *kakarot_address),
        universal_deployer: None,
        accounts: HashMap::default(),
        contracts,
    };

    // Write the genesis json to the file.
    std::fs::create_dir_all(GENESIS_FOLDER_PATH.as_path()).expect("Failed to create genesis directory");

    let genesis_path = GENESIS_FOLDER_PATH.as_path().join("genesis.json");
    std::fs::write(genesis_path, serde_json::to_string(&genesis).expect("Failed to serialize genesis json"))
        .expect("Failed to write genesis json");

    // Write the manifest to the file.
    let manifest_path = GENESIS_FOLDER_PATH.as_path().join("manifest.json");
    std::fs::write(manifest_path, serde_json::to_string(&manifest).expect("Failed to serialize manifest json"))
        .expect("Failed to write manifest json");
}

fn compute_class_hash(class_path: &Path) -> FieldElement {
    let class_code = std::fs::read_to_string(class_path).expect("Failed to read class code");
    // We try to deserialize the class as a v1 class, if it fails we try to deserialize it as a v0 class.
    match serde_json::from_str::<ContractClass>(&class_code) {
        Ok(casm) => {
            let casm = CasmContractClass::from_contract_class(casm, true).expect("Failed to convert class");
            FieldElement::from_bytes_be(&casm.compiled_class_hash().to_be_bytes())
                .expect("Failed to convert class hash")
        }
        Err(_) => {
            let casm: LegacyContractClass = serde_json::from_str(&class_code).expect("Failed to parse class code v0");
            casm.class_hash().expect("Failed to get class hash v0")
        }
    }
}

fn random_felt() -> FieldElement {
    let mut rng = SmallRng::from_seed([0u8; 32]);
    let mut rand_bytes = [0u8; 32];
    rng.fill_bytes(&mut rand_bytes);
    rand_bytes[0] %= 0x8;

    FieldElement::from_bytes_be(&rand_bytes).expect("Failed to generate random field element")
}

fn storage(var_name: &str, value: FieldElement) -> (FieldElement, FieldElement) {
    (get_storage_var_address(var_name, &[]).expect("Failed to compute storage address"), value)
}

fn set_up_kakarot(class_hashes: &HashMap<String, FieldElement>) -> (ContractAddress, GenesisContractJson) {
    // Read the env var or generate a random field element
    let kakarot_address = ContractAddress::new(random_felt());

    // Construct the kakarot contract storage.
    let kakarot_storage = [
        storage("native_token_address", *DEFAULT_FEE_TOKEN_ADDRESS),
        storage(
            "contract_account_class_hash",
            class_hashes.get("contract_account").cloned().expect("Failed to get contract_account class hash"),
        ),
        storage(
            "externally_owned_account_class_hash",
            class_hashes.get("externally_owned_account").cloned().expect("Failed to get contract_account class hash"),
        ),
        storage(
            "account_proxy_class_hash",
            class_hashes.get("proxy").cloned().expect("Failed to get proxy class hash"),
        ),
        storage("deploy_fee", FieldElement::ZERO),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();

    // Construct the kakarot contract.
    (
        kakarot_address,
        GenesisContractJson {
            class: class_hashes.get("kakarot").cloned().expect("Failed to get Kakarot class hash"),
            balance: None,
            nonce: None,
            storage: Some(kakarot_storage),
        },
    )
}

fn set_up_eoa(
    class_hashes: &HashMap<String, FieldElement>,
    kakarot_address: FieldElement,
    proxy_account_class_hash: FieldElement,
) -> (ContractAddress, GenesisContractJson) {
    // Set up the EOA
    let pk = B256::from_str(&var("EVM_PRIVATE_KEY").expect("Missing EVM private key"))
        .expect("Failed to parse EVM private key");
    let wallet = LocalWallet::from_bytes(pk.as_slice()).expect("Failed to create wallet");
    let eoa_evm_address = wallet.address();
    let eoa_evm_address_bytes = eoa_evm_address.as_bytes();
    let eoa_evm_address = FieldElement::from_byte_slice_be(eoa_evm_address_bytes).expect("Failed to convert address");
    let eoa_address = ContractAddress::new(starknet_address(
        Address::from_slice(eoa_evm_address_bytes),
        kakarot_address,
        proxy_account_class_hash,
    ));

    // Construct the EOA storage.
    let eoa_storage = [
        storage("evm_address", eoa_evm_address),
        storage("kakarot_address", kakarot_address),
        storage(
            "_implementation",
            class_hashes.get("externally_owned_account").cloned().expect("Failed to get EOA class hash"),
        ),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();

    // Construct the EOA contract.
    (
        eoa_address,
        GenesisContractJson {
            class: class_hashes.get("proxy").cloned().expect("Failed to get the proxy class hash"),
            balance: Some(U256::MAX),
            nonce: None,
            storage: Some(eoa_storage),
        },
    )
}

fn set_up_fee_token(eoa_address: FieldElement, kakarot_address: FieldElement) -> FeeTokenConfigJson {
    let key = get_storage_var_address("ERC20_allowances", &[eoa_address, kakarot_address])
        .expect("Failed to compute allowances storage address");
    let storage = [(key, FieldElement::from(u128::MAX)), (key + FieldElement::ONE, FieldElement::from(u128::MAX))]
        .iter()
        .cloned()
        .collect();

    FeeTokenConfigJson {
        name: "Ethereum".to_string(),
        symbol: "ETH".to_string(),
        decimals: 18,
        address: None,
        class: None,
        storage: Some(storage),
    }
}

fn starknet_address(
    address: Address,
    kakarot_address: FieldElement,
    proxy_account_class_hash: FieldElement,
) -> FieldElement {
    get_contract_address(into_via_wrapper!(address), proxy_account_class_hash, &[], kakarot_address)
}
