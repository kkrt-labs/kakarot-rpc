use dotenv::dotenv;
use ethers::types::U256;
use kakarot_rpc::test_utils::katana::genesis::KatanaGenesisBuilder;
use lazy_static::lazy_static;
use reth_primitives::B256;
use starknet_crypto::FieldElement;
use std::{
    env::var,
    path::{Path, PathBuf},
    str::FromStr,
};

lazy_static! {
    static ref GENESIS_FOLDER_PATH: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join(".katana");
    static ref KAKAROT_CONTRACTS_PATH: PathBuf =
        Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join("lib/kakarot/build");
    static ref COINBASE_ADDRESS: FieldElement = FieldElement::from(0x12345u32);
    static ref SALT: FieldElement = FieldElement::ZERO;
}

fn main() {
    // Load the env vars.
    dotenv().ok();

    // Read the env vars.
    let pk = B256::from_str(&var("EVM_PRIVATE_KEY").expect("Missing EVM private key"))
        .expect("Failed to parse EVM private key");

    // Read all the classes.
    let mut builder = KatanaGenesisBuilder::default()
        .load_classes(KAKAROT_CONTRACTS_PATH.clone())
        .with_kakarot(*COINBASE_ADDRESS)
        .expect("Failed to set up Kakarot");
    builder = builder.with_eoa(pk, None).expect("Failed to set up EOA").fund(pk, U256::from(u128::MAX)).unwrap();

    let manifest = builder.manifest();

    let genesis = builder.build().expect("Failed to build genesis");

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
