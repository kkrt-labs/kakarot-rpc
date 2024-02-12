use dotenv::dotenv;
use kakarot_rpc::test_utils::{hive::HiveGenesisConfig, katana::genesis::KatanaGenesisBuilder};
use std::{env::var, path::Path};

fn main() {
    // Load the env vars.
    dotenv().ok();

    let kakarot_contracts_path =
        Path::new(&var("KAKAROT_CONTRACTS_PATH").expect("Failed to load KAKAROT_CONTRACTS_PATH var")).to_path_buf();
    let hive_genesis_path =
        Path::new(&var("HIVE_GENESIS_PATH").expect("Failed to load HIVE_GENESIS_PATH var")).to_path_buf();

    // Read all the classes.
    let builder = KatanaGenesisBuilder::default().load_classes(kakarot_contracts_path);

    // Read the hive genesis.
    let hive_genesis_content = std::fs::read_to_string(hive_genesis_path).expect("Failed to read hive genesis file");
    let hive_genesis: HiveGenesisConfig =
        serde_json::from_str(&hive_genesis_content).expect("Failed to parse hive genesis json");

    // Convert the hive genesis to a katana genesis.
    let genesis =
        hive_genesis.try_into_genesis_json(builder).expect("Failed to convert hive genesis to katana genesis");

    // Write the genesis json to the file.
    let genesis_path = Path::new(&var("GENESIS_OUTPUT").expect("Failed to load GENESIS_OUTPUT var")).to_path_buf();
    std::fs::write(genesis_path, serde_json::to_string(&genesis).expect("Failed to serialize genesis json"))
        .expect("Failed to write genesis json");
}
