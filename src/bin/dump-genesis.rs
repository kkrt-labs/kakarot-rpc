use std::path::Path;

use kakarot_rpc::hive_utils::hive_genesis::{
    serialize_hive_to_madara_genesis_config, GenesisLoader, HiveGenesisConfig,
};

#[tokio::main]
async fn main() {
    // Read the hive genesis
    let hive_genesis = HiveGenesisConfig::from_file("src/hive_utils/test_data/hive_genesis.json").unwrap();

    // Read the madara genesis
    let madara_loader =
        serde_json::from_str::<GenesisLoader>(std::include_str!("../hive_utils/test_data/madara_genesis.json"))
            .unwrap();
    let combined_genesis = Path::new(".hive/genesis.json");
    let compiled_path = Path::new("cairo-contracts/kakarot");

    // Dump the genesis
    std::fs::create_dir_all(".hive/").expect("Failed to create Hive dump dir");
    serialize_hive_to_madara_genesis_config(hive_genesis, madara_loader, combined_genesis, compiled_path)
        .await
        .unwrap();
}
