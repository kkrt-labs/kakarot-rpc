use alloy_primitives::{B256, U256};
use dotenvy::dotenv;
use kakarot_rpc::test_utils::katana::genesis::KatanaGenesisBuilder;
use starknet::core::types::Felt;
use std::{
    env::var,
    path::{Path, PathBuf},
    str::FromStr,
    sync::LazyLock,
};

/// Katana genesis folder path.
static GENESIS_FOLDER_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join(".katana"));

/// Kakarot contracts path.
static KAKAROT_CONTRACTS_PATH: LazyLock<PathBuf> =
    LazyLock::new(|| Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf().join("lib/kakarot/build"));

/// Mock coinbase address.
static COINBASE_ADDRESS: LazyLock<Felt> = LazyLock::new(|| 0x12345u32.into());

static CHAIN_ID: LazyLock<Felt> = LazyLock::new(|| Felt::from_str("0xb615f74ebad2c").expect("Invalid chain ID"));

fn main() {
    // Load the env vars.
    dotenv().ok();

    // Read the env vars.
    let pk = B256::from_str(&var("EVM_PRIVATE_KEY").expect("Missing EVM private key"))
        .expect("Failed to parse EVM private key");

    // Read all the classes.
    let mut builder = KatanaGenesisBuilder::default()
        .load_classes(KAKAROT_CONTRACTS_PATH.clone())
        .with_kakarot(*COINBASE_ADDRESS, *CHAIN_ID)
        .expect("Failed to set up Kakarot");
    builder = builder.with_eoa(pk).expect("Failed to set up EOA").fund(pk, U256::from(u128::MAX)).unwrap();
    builder = builder.with_dev_allocation(10);

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
