pub mod genesis;

use std::path::Path;
use std::sync::Arc;

use dojo_test_utils::sequencer::{Environment, StarknetConfig, TestSequencer};
use katana_primitives::block::GasPrices;
use katana_primitives::chain::ChainId;
use katana_primitives::genesis::json::GenesisJson;
use katana_primitives::genesis::Genesis;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

use crate::eth_provider::provider::EthDataProvider;
use crate::test_utils::eoa::KakarotEOA;
use std::collections::HashMap;

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    super::mongo::{CollectionDB, MongoFuzzer, StoredData, DOCKER_CLI},
    dojo_test_utils::sequencer::SequencerConfig,
    reth_primitives::{TxType, B256},
    reth_rpc_types::Transaction,
    std::str::FromStr as _,
    testcontainers::{Container, GenericImage},
};

fn load_genesis() -> Genesis {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(".katana/genesis.json");
    let genesis_json = GenesisJson::load(path).expect("Failed to load genesis.json, run `make katana-genesis`");
    Genesis::try_from(genesis_json).expect("Failed to convert GenesisJson to Genesis")
}

/// Returns a `StarknetConfig` instance customized for Kakarot.
/// If `with_dumped_state` is true, the config will be initialized with the dumped state.
pub fn katana_config() -> StarknetConfig {
    let max_steps = std::u32::MAX;
    StarknetConfig {
        disable_fee: true,
        env: Environment {
            chain_id: ChainId::parse("kakatest").unwrap(),
            invoke_max_steps: max_steps,
            validate_max_steps: max_steps,
            gas_price: GasPrices { eth: 1, strk: 0 },
        },
        genesis: load_genesis(),
        ..Default::default()
    }
}

/// Returns a `TestSequencer` configured for Kakarot.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
async fn katana_sequencer() -> TestSequencer {
    TestSequencer::start(SequencerConfig { no_mining: false, block_time: None, messaging: None }, katana_config()).await
}

/// Represents the Katana test environment.
#[allow(missing_debug_implementations)]
pub struct Katana {
    /// The test sequencer instance for managing test execution.
    pub sequencer: TestSequencer,
    /// The Kakarot EOA (Externally Owned Account) instance.
    pub eoa: KakarotEOA<Arc<JsonRpcClient<HttpTransport>>>,
    /// Mock data stored in a HashMap, representing the database.
    pub mock_data: HashMap<CollectionDB, Vec<StoredData>>,
    /// The port number used for communication.
    pub port: u16,
    /// Option to store the Docker container instance.
    /// It holds `Some` when the container is running, and `None` otherwise.
    pub container: Option<Container<'static, GenericImage>>,
}

impl<'a> Katana {
    #[cfg(any(test, feature = "arbitrary", feature = "testing"))]
    pub async fn new(rnd_bytes_size: usize) -> Self {
        let sequencer = katana_sequencer().await;
        let starknet_provider = Arc::new(JsonRpcClient::new(HttpTransport::new(sequencer.url())));

        Self::initialize(sequencer, starknet_provider, rnd_bytes_size).await
    }

    /// Initializes the Katana test environment.
    #[cfg(any(test, feature = "arbitrary", feature = "testing"))]
    async fn initialize(
        sequencer: TestSequencer,
        starknet_provider: Arc<JsonRpcClient<HttpTransport>>,
        rnd_bytes_size: usize,
    ) -> Self {
        // Load the private key from the environment variables.
        dotenvy::dotenv().expect("Failed to load .env file");
        let pk = std::env::var("EVM_PRIVATE_KEY").expect("Failed to get EVM private key");
        let pk = B256::from_str(&pk).expect("Failed to parse EVM private key");

        // Initialize a MongoFuzzer instance with the specified random bytes size.
        let mut mongo_fuzzer = MongoFuzzer::new(rnd_bytes_size).await;
        // Get the port number for communication.
        let port = mongo_fuzzer.port();

        // Run a Docker container with the MongoDB image.
        let container = DOCKER_CLI.run(mongo_fuzzer.mongo_image());

        // Add random transactions to the MongoDB database.
        mongo_fuzzer.add_random_transactions(10).expect("Failed to add documents in the database");
        // Add a hardcoded block header range to the MongoDB database.
        mongo_fuzzer.add_hardcoded_block_header_range(0..4).expect("Failed to add block range in the database");
        // Add a hardcoded Eip1559 transaction to the MongoDB database.
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Eip1559))
            .expect("Failed to add Eip1559 transaction in the database");
        // Add a hardcoded Eip2930 transaction to the MongoDB database.
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Eip2930))
            .expect("Failed to add Eip2930 transaction in the database");
        // Add a hardcoded Legacy transaction to the MongoDB database.
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Legacy))
            .expect("Failed to add Legacy transaction in the database");
        // Finalize the MongoDB database initialization and get the database instance.
        let database = mongo_fuzzer.finalize().await;
        // Clone the mock data stored in the MongoFuzzer instance.
        let mock_data = (*mongo_fuzzer.documents()).clone();

        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let eth_provider = Arc::new(
            EthDataProvider::new(database, starknet_provider).await.expect("Failed to create EthDataProvider"),
        );

        // Create a new Kakarot EOA instance with the private key and EthDataProvider instance.
        let eoa = KakarotEOA::new(pk, eth_provider);

        // Return a new instance of Katana with initialized fields.
        Self { sequencer, eoa, mock_data, port, container: Some(container) }
    }

    pub fn eth_provider(&self) -> Arc<EthDataProvider<Arc<JsonRpcClient<HttpTransport>>>> {
        self.eoa.eth_provider.clone()
    }

    pub fn eoa(&self) -> KakarotEOA<Arc<JsonRpcClient<HttpTransport>>> {
        self.eoa.clone()
    }

    /// allow(dead_code) is used because this function is used in tests,
    /// and each test is compiled separately, so the compiler thinks this function is unused
    #[allow(dead_code)]
    pub const fn sequencer(&self) -> &TestSequencer {
        &self.sequencer
    }

    /// Retrieves the first stored transaction
    pub fn first_transaction(&self) -> Option<Transaction> {
        self.mock_data
            .get(&CollectionDB::Transactions)
            .and_then(|transactions| transactions.first())
            .and_then(|data| data.extract_stored_transaction())
            .map(|stored_tx| stored_tx.tx.clone())
    }

    /// Retrieves the most recent stored transaction based on block number
    pub fn most_recent_transaction(&self) -> Option<Transaction> {
        self.mock_data
            .get(&CollectionDB::Transactions)
            .and_then(|transactions| {
                transactions.iter().max_by_key(|data| {
                    data.extract_stored_transaction().map(|stored_tx| stored_tx.tx.block_number).unwrap_or_default()
                })
            })
            .and_then(|data| data.extract_stored_transaction())
            .map(|stored_tx| stored_tx.tx.clone())
    }

    /// Retrieves the number of blocks in the database
    pub fn count_block(&self) -> usize {
        self.mock_data.get(&CollectionDB::Headers).map_or(0, |headers| headers.len())
    }
}
