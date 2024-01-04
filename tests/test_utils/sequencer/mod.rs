use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use crate::test_utils::eoa::{Eoa, KakarotEOA};
use dojo_test_utils::sequencer::{Environment, SequencerConfig, StarknetConfig, TestSequencer};
use ethers::types::H256;
use foundry_config::utils::find_project_root_path;
use kakarot_rpc::starknet_client::config::{KakarotRpcConfig, Network};
use kakarot_rpc::starknet_client::KakarotClient;
use katana_core::db::serde::state::SerializableState;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet_crypto::FieldElement;

use crate::root_project_path;

/// Returns the dumped Katana state with deployed Kakarot.
pub fn load_katana_state() -> SerializableState {
    // Get dump path
    let path = root_project_path!(".katana/dump.bin");

    // Load Serializable state from path
    SerializableState::load(path).expect("Failed to load Katana state")
}

/// Returns a `StarknetConfig` instance customized for Kakarot.
/// If `with_dumped_state` is true, the config will be initialized with the dumped state.
pub fn katana_config() -> StarknetConfig {
    let max_steps = std::u32::MAX;
    StarknetConfig {
        disable_fee: true,
        env: Environment {
            chain_id: "SN_GOERLI".into(),
            invoke_max_steps: max_steps,
            validate_max_steps: max_steps,
            gas_price: 1,
        },
        init_state: Some(load_katana_state()),
        ..Default::default()
    }
}

/// Returns a `TestSequencer` configured for Kakarot.
async fn katana_sequencer() -> TestSequencer {
    TestSequencer::start(SequencerConfig { no_mining: false, block_time: None }, katana_config()).await
}

pub struct Katana {
    pub sequencer: TestSequencer,
    pub eoa: KakarotEOA<JsonRpcClient<HttpTransport>>,
}

impl Katana {
    pub async fn new() -> Self {
        let sequencer = katana_sequencer().await;
        let starknet_provider = Arc::new(JsonRpcClient::new(HttpTransport::new(sequencer.url())));

        Self::initialize(sequencer, starknet_provider)
    }

    /// Initializes the Katana test environment.
    fn initialize(sequencer: TestSequencer, starknet_provider: Arc<JsonRpcClient<HttpTransport>>) -> Self {
        // Load deployments
        let deployments_path = root_project_path!("lib/kakarot/deployments/katana/deployments.json");
        let deployments = std::fs::read_to_string(deployments_path).expect("Failed to read deployment file");
        let deployments: HashMap<&str, serde_json::Value> =
            serde_json::from_str(&deployments).expect("Failed to deserialize deployments");

        let kakarot_address = deployments["kakarot"]["address"].as_str().expect("Failed to get Kakarot address");
        let kakarot_address = FieldElement::from_hex_be(kakarot_address).expect("Failed to parse Kakarot address");

        // Load declarations
        let declaration_path = root_project_path!("lib/kakarot/deployments/katana/declarations.json");
        let declarations = std::fs::read_to_string(declaration_path).expect("Failed to read declaration file");
        let declarations: HashMap<&str, FieldElement> =
            serde_json::from_str(&declarations).expect("Failed to deserialize declarations");

        let proxy_class_hash = declarations["proxy"];
        let externally_owned_account_class_hash = declarations["externally_owned_account"];
        let contract_account_class_hash = declarations["contract_account"];

        // Load PK
        dotenv::dotenv().expect("Failed to load .env file");
        let pk = std::env::var("EVM_PRIVATE_KEY").expect("Failed to get EVM private key");
        let pk = H256::from_str(&pk).expect("Failed to parse EVM private key").into();

        // Create a Kakarot client
        let kakarot_client = KakarotClient::new(
            KakarotRpcConfig::new(
                Network::JsonRpcProvider(sequencer.url()),
                kakarot_address,
                proxy_class_hash,
                externally_owned_account_class_hash,
                contract_account_class_hash,
            ),
            starknet_provider,
        );

        let eoa = KakarotEOA::new(pk, kakarot_client);

        Self { sequencer, eoa }
    }

    pub const fn eoa(&self) -> &KakarotEOA<JsonRpcClient<HttpTransport>> {
        &self.eoa
    }

    pub fn client(&self) -> &KakarotClient<JsonRpcClient<HttpTransport>> {
        self.eoa.client()
    }

    /// allow(dead_code) is used because this function is used in tests,
    /// and each test is compiled separately, so the compiler thinks this function is unused
    #[allow(dead_code)]
    pub const fn sequencer(&self) -> &TestSequencer {
        &self.sequencer
    }
}
