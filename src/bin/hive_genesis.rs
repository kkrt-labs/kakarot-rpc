use clap::Parser;
use kakarot_rpc::test_utils::{hive::HiveGenesisConfig, katana::genesis::KatanaGenesisBuilder};
use starknet::core::types::Felt;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[clap(long, short)]
    kakarot_contracts: PathBuf,
    #[clap(long)]
    hive_genesis: PathBuf,
    #[clap(long, short)]
    genesis_out: PathBuf,
    #[clap(long, short)]
    manifest_out: PathBuf,
}

fn main() {
    let args = Args::parse();
    let kakarot_contracts_path = args.kakarot_contracts;
    let hive_genesis_path = args.hive_genesis;
    let genesis_path = args.genesis_out;
    let manifest_path = args.manifest_out;

    // Read all the classes.
    let mut builder = KatanaGenesisBuilder::default().load_classes(kakarot_contracts_path);

    // Add dev allocations.
    builder = builder.with_dev_allocation(10);

    // Read the hive genesis.
    let hive_genesis_content = std::fs::read_to_string(hive_genesis_path).expect("Failed to read hive genesis file");
    let hive_genesis: HiveGenesisConfig =
        serde_json::from_str(&hive_genesis_content).expect("Failed to parse hive genesis json");

    // Convert the hive genesis to a katana genesis.
    let genesis_json =
        hive_genesis.try_into_genesis_json(builder.clone()).expect("Failed to convert hive genesis to katana genesis");

    let builder = builder.with_kakarot(Felt::ZERO).expect("Failed to set up Kakarot");
    let manifest = builder.manifest();

    // Write the genesis json to the file.
    std::fs::write(genesis_path, serde_json::to_string(&genesis_json).expect("Failed to serialize genesis json"))
        .expect("Failed to write genesis json");

    // Write the manifest to the file.
    std::fs::write(manifest_path, serde_json::to_string(&manifest).expect("Failed to serialize manifest json"))
        .expect("Failed to write manifest json");
}
