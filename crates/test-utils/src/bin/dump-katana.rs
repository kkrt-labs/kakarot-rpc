use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use git2::{Repository, SubmoduleIgnore};
use kakarot_rpc_core::client::api::KakarotStarknetApi;
use kakarot_test_utils::constants::STARKNET_DEPLOYER_ACCOUNT_PRIVATE_KEY;
use kakarot_test_utils::deploy_helpers::{DeployerAccount, KakarotTestEnvironmentContext};
use starknet::accounts::Account;

#[tokio::main]
async fn main() {
    // Deploy all kakarot contracts + EVM contracts
    let with_dumped_state = false;
    let test_context = KakarotTestEnvironmentContext::new(with_dumped_state).await;

    // Get a serializable state for the sequencer
    let sequencer = test_context.sequencer();
    let dump_state = sequencer
        .sequencer
        .backend
        .state
        .write()
        .await
        .dump_state()
        .expect("Failed to call dump_state on Katana state");

        // Dump the state
        std::fs::create_dir_all(".katana/").expect("Failed to create Kakata dump dir");

        let katana_dump_path = String::from(".katana/dump.json");
        let katana_dump_file = File::options()
            .create_new(true)
            .read(true)
            .write(true)
            .append(false)
            .open(katana_dump_path)
            .expect(format!("Failed to open file {}", katana_dump_path));
        serde_json::to_writer_pretty(katana_dump_file, &dump_state)
            .expect(format!("Failed to write to the file {}", katana_dump_path));

    let deployer_account = DeployerAccount {
        address: test_context.client().deployer_account().address(),
        private_key: *STARKNET_DEPLOYER_ACCOUNT_PRIVATE_KEY,
    };

    // Store contracts information
    let mut contracts = HashMap::new();
    contracts.insert("Kakarot", serde_json::to_value(test_context.kakarot()).unwrap());
    contracts.insert("ERC20", serde_json::to_value(test_context.evm_contract("ERC20")).unwrap());
    contracts.insert("Counter", serde_json::to_value(test_context.evm_contract("Counter")).unwrap());
    contracts.insert("PlainOpcodes", serde_json::to_value(test_context.evm_contract("PlainOpcodes")).unwrap());
    contracts.insert("DeployerAccount", serde_json::to_value(deployer_account).unwrap());

    // Dump the contracts information
    let contracts = serde_json::to_string(&contracts).expect("Failed to serialize contract addresses");
    std::fs::write(".katana/contracts.json", contracts)
        .expect("Failed to write contracts informations to .katana/contracts.json");

    // Get the sha of the kakarot submodule
    let repo = Repository::open(".").unwrap();
    let kakarot_submodule = repo.find_submodule("kakarot").expect("Failed to find kakarot submodule");
    let sha = kakarot_submodule.index_id().unwrap_or_else(|| kakarot_submodule.head_id().unwrap()).to_string();

    // Check if the submodule is dirty
    let kakarot_submodule_status =
        repo.submodule_status("kakarot", SubmoduleIgnore::None).expect("Failed to get kakarot submodule status");
    let is_submodule_workdir_dirty = kakarot_submodule_status.is_wd_wd_modified();
    let sha = if is_submodule_workdir_dirty { sha + "-dirty" } else { sha };
    std::fs::write(".katana/kakarot_sha", sha).expect("Failed to write submodules to .katana/kakarot_sha");
}
