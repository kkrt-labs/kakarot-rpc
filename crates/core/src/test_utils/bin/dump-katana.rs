use std::collections::HashMap;

use ethers::abi::Token;
use git2::Repository;
use kakarot_rpc_core::client::api::KakarotStarknetApi;
use kakarot_rpc_core::test_utils::constants::STARKNET_DEPLOYER_ACCOUNT_PRIVATE_KEY;
use kakarot_rpc_core::test_utils::deploy_helpers::{
    ContractDeploymentArgs, DeployerAccount, KakarotTestEnvironmentContext, TestContext,
};
use starknet::accounts::Account;

#[tokio::main]
async fn main() {
    // Deploy all kakarot contracts + EVM contracts
    let mut test_context = KakarotTestEnvironmentContext::new(TestContext::PlainOpcodes).await;
    test_context = test_context
        .deploy_evm_contract(ContractDeploymentArgs {
            name: "ERC20".into(),
            constructor_args: (
                Token::String("Test".into()),               // name
                Token::String("TT".into()),                 // symbol
                Token::Uint(ethers::types::U256::from(18)), // decimals
            ),
        })
        .await;

    tokio::task::spawn_blocking(move || {
        // Get a serializable state for the sequencer
        let sequencer = test_context.sequencer();
        let dump_state = sequencer
            .sequencer
            .backend
            .state
            .blocking_write()
            .dump_state()
            .expect("Failed to call dump_state on Katana state");

        let state = serde_json::to_string(&dump_state).expect("Failed to serialize state");

        // Dump the state
        std::fs::create_dir_all(".katana/").expect("Failed to create Kakata dump dir");
        std::fs::write(".katana/dump.json", state).expect("Failed to write dump to .katana/dump.json");

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
    })
    .await
    .expect("Failed to dump state");

    let repo = Repository::open(".").unwrap();
    let submodules: Vec<String> = repo
        .submodules()
        .unwrap()
        .iter()
        .filter(|submodule| submodule.name().unwrap() == "kakarot")
        .map(|submodule| submodule.workdir_id().unwrap_or_else(|| submodule.head_id().unwrap()).to_string())
        .collect();
    std::fs::write(".katana/kakarot_sha", &submodules[0]).expect("Failed to write submodules to .katana/kakarot_sha");
}
