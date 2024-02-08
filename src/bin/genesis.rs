use dotenv::dotenv;
use kakarot_rpc::test_utils::katana::genesis::KatanaGenesisBuilder;
use lazy_static::lazy_static;
use reth_primitives::B256;
use serde::Serialize;
use serde_with::serde_as;
use starknet::core::serde::unsigned_field_element::UfeHex;
use starknet_crypto::FieldElement;
use std::{
    collections::HashMap,
    env::var,
    path::{Path, PathBuf},
    str::FromStr,
};

lazy_static! {
    static ref GENESIS_FOLDER_PATH: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join(".katana");
    static ref KAKAROT_CONTRACTS_RELATIVE_PATH: PathBuf = GENESIS_FOLDER_PATH.join("../lib/kakarot/build");
    static ref INCORRECT_FELT: String = "INCORRECT".to_string();
    static ref COINBASE_ADDRESS: FieldElement = FieldElement::from(0x12345u32);
    static ref SALT: FieldElement = FieldElement::from_bytes_be(&[0u8; 32]).expect("Failed to convert salt");
}

#[serde_as]
#[derive(Serialize)]
struct Hex(#[serde_as(as = "UfeHex")] pub FieldElement);

#[derive(Serialize)]
struct KatanaManifest {
    declarations: HashMap<String, Hex>,
    deployments: HashMap<String, Hex>,
}

fn main() {
    // Load the env vars.
    dotenv().ok();

    // Read the env vars.
    let pk = B256::from_str(&var("EVM_PRIVATE_KEY").expect("Missing EVM private key"))
        .expect("Failed to parse EVM private key");

    // Read all the classes.
    let mut builder = KatanaGenesisBuilder::new()
        .load_classes(KAKAROT_CONTRACTS_RELATIVE_PATH.clone())
        .with_kakarot()
        .expect("Failed to set up Kakarot");
    builder = builder.with_eoa(pk).expect("Failed to set up EOA");

    // Compute the coinbase address.
    let sequencer_address = builder.compute_starknet_address(*COINBASE_ADDRESS).unwrap();

    let cache = builder.cache().clone().into_iter().map(|(k, v)| (k, Hex(v))).collect::<HashMap<_, _>>();
    let class_hashes = builder.class_hashes().clone().into_iter().map(|(k, v)| (k, Hex(v))).collect::<HashMap<_, _>>();
    let manifest = KatanaManifest { declarations: class_hashes, deployments: cache };

    let genesis = builder.build(sequencer_address).expect("Failed to build genesis");

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
