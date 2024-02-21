use dotenv::dotenv;
use kakarot_rpc::test_utils::{hive::HiveGenesisConfig, katana::genesis::KatanaGenesisBuilder};
use katana_primitives::genesis::{
    allocation::DevAllocationsGenerator, constant::DEFAULT_PREFUNDED_ACCOUNT_BALANCE, json::GenesisAccountJson,
};
use starknet_crypto::FieldElement;
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
    let mut genesis_json =
        hive_genesis.try_into_genesis_json(builder.clone()).expect("Failed to convert hive genesis to katana genesis");

    // Add dev allocations.
    let dev_allocations = DevAllocationsGenerator::new(10)
        .with_balance(DEFAULT_PREFUNDED_ACCOUNT_BALANCE)
        .generate()
        .into_iter()
        .map(|(address, account)| {
            (
                address,
                GenesisAccountJson {
                    public_key: account.public_key,
                    balance: Some(account.balance),
                    nonce: account.nonce,
                    class: None,
                    storage: account.storage.clone(),
                },
            )
        })
        .collect::<Vec<_>>();
    genesis_json.accounts.extend(dev_allocations);

    let builder = builder.with_kakarot(FieldElement::ZERO).expect("Failed to set up Kakarot");
    let manifest = builder.manifest();

    // Write the genesis json to the file.
    let genesis_path = Path::new(&var("GENESIS_OUTPUT").expect("Failed to load GENESIS_OUTPUT var")).to_path_buf();
    std::fs::write(genesis_path, serde_json::to_string(&genesis_json).expect("Failed to serialize genesis json"))
        .expect("Failed to write genesis json");

    // Write the manifest to the file.
    let manifest_path = Path::new(&var("MANIFEST_OUTPUT").expect("Failed to load MANIFEST_OUTPUT var")).to_path_buf();
    std::fs::write(manifest_path, serde_json::to_string(&manifest).expect("Failed to serialize manifest json"))
        .expect("Failed to write manifest json");
}
