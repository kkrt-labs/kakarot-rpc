pub mod genesis;

use std::path::Path;
use std::str::FromStr as _;
use std::sync::Arc;

use dojo_test_utils::sequencer::{Environment, SequencerConfig, StarknetConfig, TestSequencer};
use katana_primitives::chain::ChainId;
use katana_primitives::genesis::allocation::{DevAllocationsGenerator, GenesisAllocation};
use katana_primitives::genesis::json::GenesisJson;
use katana_primitives::genesis::Genesis;
use reth_primitives::B256;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet_crypto::FieldElement;

use crate::eth_provider::provider::EthDataProvider;
use crate::test_utils::eoa::KakarotEOA;

use super::mongo::mock_database;

fn load_genesis() -> Genesis {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".katana/genesis.json");
    let genesis_json = GenesisJson::load(path).expect("Failed to load genesis.json, run `make katana-genesis`");
    let mut genesis = Genesis::try_from(genesis_json).expect("Failed to convert GenesisJson to Genesis");

    let dev_allocations = DevAllocationsGenerator::new(10)
        .generate()
        .into_iter()
        .map(|(address, account)| (address, Into::<GenesisAllocation>::into(account)))
        .collect::<Vec<_>>();
    genesis.extend_allocations(dev_allocations);

    genesis
}

/// Returns a `StarknetConfig` instance customized for Kakarot.
/// If `with_dumped_state` is true, the config will be initialized with the dumped state.
pub fn katana_config() -> StarknetConfig {
    let max_steps = std::u32::MAX;
    StarknetConfig {
        disable_fee: true,
        env: Environment {
            chain_id: ChainId::Id(FieldElement::from(0x4b4b5254u32)),
            invoke_max_steps: max_steps,
            validate_max_steps: max_steps,
            gas_price: 1,
        },
        genesis: load_genesis(),
        ..Default::default()
    }
}

/// Returns a `TestSequencer` configured for Kakarot.
async fn katana_sequencer() -> TestSequencer {
    TestSequencer::start(SequencerConfig { no_mining: false, block_time: None, messaging: None }, katana_config()).await
}

pub struct Katana {
    pub sequencer: TestSequencer,
    pub eoa: KakarotEOA<Arc<JsonRpcClient<HttpTransport>>>,
}

impl Katana {
    pub async fn new() -> Self {
        let sequencer = katana_sequencer().await;
        let starknet_provider = Arc::new(JsonRpcClient::new(HttpTransport::new(sequencer.url())));

        Self::initialize(sequencer, starknet_provider).await
    }

    /// Initializes the Katana test environment.
    async fn initialize(sequencer: TestSequencer, starknet_provider: Arc<JsonRpcClient<HttpTransport>>) -> Self {
        // Load PK
        dotenv::dotenv().expect("Failed to load .env file");
        let pk = std::env::var("EVM_PRIVATE_KEY").expect("Failed to get EVM private key");
        let pk = B256::from_str(&pk).expect("Failed to parse EVM private key");

        // Create a Kakarot client
        let database = mock_database().await;
        let eth_provider = Arc::new(EthDataProvider::new(database, starknet_provider));

        let eoa = KakarotEOA::new(pk, eth_provider);

        Self { sequencer, eoa }
    }

    pub fn eth_provider(&self) -> Arc<EthDataProvider<Arc<JsonRpcClient<HttpTransport>>>> {
        self.eoa.eth_provider.clone()
    }

    pub const fn eoa(&self) -> &KakarotEOA<Arc<JsonRpcClient<HttpTransport>>> {
        &self.eoa
    }

    /// allow(dead_code) is used because this function is used in tests,
    /// and each test is compiled separately, so the compiler thinks this function is unused
    #[allow(dead_code)]
    pub const fn sequencer(&self) -> &TestSequencer {
        &self.sequencer
    }
}
