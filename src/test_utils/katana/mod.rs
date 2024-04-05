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
    super::mongo::{CollectionDB, MongoFuzzer, StoredData},
    crate::eth_provider::database::types::transaction::StoredTransaction,
    dojo_test_utils::sequencer::SequencerConfig,
    reth_primitives::{TxType, B256, U64},
    std::str::FromStr as _,
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

pub struct Katana {
    pub sequencer: TestSequencer,
    pub eoa: KakarotEOA<Arc<JsonRpcClient<HttpTransport>>>,
    pub mock_data: HashMap<CollectionDB, Vec<StoredData>>,
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
        // Load PK
        dotenvy::dotenv().expect("Failed to load .env file");
        let pk = std::env::var("EVM_PRIVATE_KEY").expect("Failed to get EVM private key");
        let pk = B256::from_str(&pk).expect("Failed to parse EVM private key");

        // Create a Kakarot client
        let mut mongo_fuzzer = MongoFuzzer::new(rnd_bytes_size).await;
        mongo_fuzzer.add_random_transactions(10).expect("Failed to add documents in the database");
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Eip1559))
            .expect("Failed to add Eip1559 transaction in the database");
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Eip2930))
            .expect("Failed to add Eip2930 transaction in the database");
        mongo_fuzzer
            .add_hardcoded_transaction(Some(TxType::Legacy))
            .expect("Failed to add Legacy transaction in the database");
        let database = mongo_fuzzer.finalize().await;
        let mock_data = (*mongo_fuzzer.documents()).clone();

        let eth_provider = Arc::new(
            EthDataProvider::new(database, starknet_provider).await.expect("Failed to create EthDataProvider"),
        );

        let eoa = KakarotEOA::new(pk, eth_provider);

        Self { sequencer, eoa, mock_data }
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

    /// Retrieves the first stored transaction of the specified type from the `Transactions` collection in the MongoDB database.
    pub fn get_transaction(&self, tx_type: TxType) -> Option<StoredTransaction> {
        // Retrieve the vector of stored data associated with the `Transactions` collection.
        if let Some(transactions) = self.mock_data.get(&CollectionDB::Transactions) {
            let tx_type = U64::from(Into::<u8>::into(tx_type));
            // Iterate through the stored data to find the first transaction of the specified type.
            for data in transactions {
                // Match the stored data to find a stored transaction.
                if let StoredData::StoredTransaction(transaction) = data {
                    // Check if the transaction type matches the specified type.
                    if let Some(transaction_type) = transaction.tx.transaction_type {
                        if transaction_type == tx_type {
                            // Return the stored transaction if the transaction type matches.
                            return Some(transaction.clone());
                        }
                    }
                }
            }
        }
        // Return None if no transaction of the specified type is found.
        None
    }
}
