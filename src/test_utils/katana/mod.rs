pub mod genesis;

use super::mongo::MongoImage;
use crate::{
    providers::eth_provider::{
        constant::U64_HEX_STRING_LEN,
        database::{
            ethereum::EthereumTransactionStore,
            filter::{self, format_hex, EthDatabaseFilterBuilder},
            types::{
                header::StoredHeader, log::StoredLog, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
            },
            CollectionName,
        },
        provider::EthDataProvider,
    },
    test_utils::eoa::KakarotEOA,
};
use dojo_test_utils::sequencer::{Environment, StarknetConfig, TestSequencer};
use katana_primitives::{
    chain::ChainId,
    genesis::{json::GenesisJson, Genesis},
};
use mongodb::{
    bson,
    bson::{doc, Document},
    options::{UpdateModifications, UpdateOptions},
};
use reth_primitives::{Address, Bytes};
use reth_rpc_types::Log;
use starknet::providers::{jsonrpc::HttpTransport, JsonRpcClient};
use std::{path::Path, sync::Arc};
use testcontainers::ContainerAsync;
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    super::mongo::MongoFuzzer,
    dojo_test_utils::sequencer::SequencerConfig,
    reth_primitives::B256,
    reth_rpc_types::{Header, Transaction},
    std::str::FromStr as _,
};

fn load_genesis() -> Genesis {
    Genesis::try_from(
        GenesisJson::load(Path::new(env!("CARGO_MANIFEST_DIR")).join(".katana/genesis.json"))
            .expect("Failed to load genesis.json, run `make katana-genesis`"),
    )
    .expect("Failed to convert GenesisJson to Genesis")
}

/// Returns a `StarknetConfig` instance customized for Kakarot.
/// If `with_dumped_state` is true, the config will be initialized with the dumped state.
pub fn katana_config() -> StarknetConfig {
    let max_steps = u32::MAX;
    StarknetConfig {
        disable_fee: true,
        env: Environment {
            // Since kaka_test > u32::MAX, we should return the last 4 bytes of the chain_id: test
            chain_id: ChainId::parse("kaka_test").unwrap(),
            invoke_max_steps: max_steps,
            validate_max_steps: max_steps,
        },
        genesis: load_genesis(),
        ..Default::default()
    }
}

/// Returns a `TestSequencer` configured for Kakarot.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
pub async fn katana_sequencer() -> TestSequencer {
    TestSequencer::start(SequencerConfig { no_mining: false, block_time: None }, katana_config()).await
}

/// Represents the Katana test environment.
#[allow(missing_debug_implementations)]
pub struct Katana {
    /// The test sequencer instance for managing test execution.
    pub sequencer: TestSequencer,
    /// The Kakarot EOA (Externally Owned Account) instance.
    pub eoa: KakarotEOA<Arc<JsonRpcClient<HttpTransport>>>,
    /// Stored headers to insert into the headers collection.
    pub headers: Vec<StoredHeader>,
    /// Stored transactions to insert into the transactions collection.
    pub transactions: Vec<StoredTransaction>,
    /// Stored transaction receipts to insert into the receipts collection.
    pub receipts: Vec<StoredTransactionReceipt>,
    /// Stored logs to insert into the logs collection.
    pub logs: Vec<StoredLog>,
    // /// The port number used for communication.
    // pub port: u16,
    /// Option to store the Docker container instance.
    /// It holds `Some` when the container is running, and `None` otherwise.
    pub container: Option<ContainerAsync<MongoImage>>,
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

        // Add random transactions to the MongoDB database.
        mongo_fuzzer.add_random_transactions(10).expect("Failed to add documents in the database");
        // Add a hardcoded logs to the MongoDB database.
        mongo_fuzzer.add_random_logs(2).expect("Failed to logs in the database");
        // Add a hardcoded header to the MongoDB database.
        let max_block_number =
            mongo_fuzzer.headers.iter().map(|header| header.number.unwrap_or_default()).max().unwrap_or_default();
        mongo_fuzzer
            .add_hardcoded_block_header_with_base_fee(max_block_number + 1, 0)
            .expect("Failed to add header in the database");

        // Finalize the MongoDB database initialization and get the database instance.
        let database = mongo_fuzzer.finalize().await;

        // Create a new EthDataProvider instance with the initialized database and Starknet provider.
        let eth_provider = Arc::new(
            EthDataProvider::new(database, starknet_provider).await.expect("Failed to create EthDataProvider"),
        );

        // Create a new Kakarot EOA instance with the private key and EthDataProvider instance.
        let eoa = KakarotEOA::new(pk, eth_provider);

        // Return a new instance of Katana with initialized fields.
        Self {
            sequencer,
            eoa,
            container: Some(mongo_fuzzer.container),
            transactions: mongo_fuzzer.transactions,
            receipts: mongo_fuzzer.receipts,
            logs: mongo_fuzzer.logs,
            headers: mongo_fuzzer.headers,
        }
    }

    pub fn eth_provider(&self) -> Arc<EthDataProvider<Arc<JsonRpcClient<HttpTransport>>>> {
        self.eoa.eth_provider.clone()
    }

    pub fn starknet_provider(&self) -> Arc<JsonRpcClient<HttpTransport>> {
        self.eoa.eth_provider.starknet_provider().clone()
    }

    pub fn eoa(&self) -> KakarotEOA<Arc<JsonRpcClient<HttpTransport>>> {
        self.eoa.clone()
    }

    #[allow(dead_code)]
    pub const fn sequencer(&self) -> &TestSequencer {
        &self.sequencer
    }

    /// Adds mock logs to the database.
    pub async fn add_mock_logs(&self, n_logs: usize) {
        // Get the Ethereum provider instance.
        let provider = self.eth_provider();

        // Get the database instance from the provider.
        let database = provider.database();

        // Create a mock log object with predefined values.
        let log = Log {
            inner: alloy_primitives::Log {
                address: Address::with_last_byte(0x69),
                data: alloy_primitives::LogData::new_unchecked(
                    vec![B256::with_last_byte(0x69)],
                    Bytes::from_static(&[0x69]),
                ),
            },
            block_hash: Some(B256::with_last_byte(0x69)),
            block_number: Some(0x69),
            block_timestamp: None,
            transaction_hash: Some(B256::with_last_byte(0x69)),
            transaction_index: Some(0x69),
            log_index: Some(0x69),
            removed: false,
        };

        // Create a vector to hold all the BSON documents to be inserted
        let log_docs: Vec<Document> = std::iter::repeat(log.clone())
            .take(n_logs)
            .map(|log| {
                let stored_log = StoredLog { log };
                bson::to_document(&stored_log).expect("Failed to serialize StoredLog to BSON")
            })
            .collect();

        // Insert all the BSON documents into the MongoDB collection at once.
        database
            .inner()
            .collection(StoredLog::collection_name())
            .insert_many(log_docs)
            .await
            .expect("Failed to insert logs into the database");
    }

    /// Adds pending transactions to the database.
    pub async fn add_pending_transactions_to_database(&self, txs: Vec<Transaction>) {
        let provider = self.eth_provider();
        let database = provider.database();

        // Add the transactions to the database.
        for tx in txs {
            database.upsert_pending_transaction(tx, 0).await.expect("Failed to update pending transaction in database");
        }
    }

    /// Adds transactions to the database along with a corresponding header.
    pub async fn add_transactions_with_header_to_database(&self, txs: Vec<Transaction>, header: Header) {
        let provider = self.eth_provider();
        let database = provider.database();
        let Header { number, .. } = header;
        let block_number = number.expect("Failed to get block number");

        // Add the transactions to the database.
        let tx_collection = database.collection::<StoredTransaction>();
        for tx in txs {
            database.upsert_transaction(tx).await.expect("Failed to update transaction in database");
        }

        // We use the unpadded block number to filter the transactions in the database and
        // the padded block number to update the block number in the database.
        let unpadded_block_number = format_hex(block_number, 0);
        let padded_block_number = format_hex(block_number, U64_HEX_STRING_LEN);

        // The transactions get added in the database with the unpadded block number (due to U256 serialization using `human_readable`).
        // We need to update the block number to the padded version.
        tx_collection
            .update_many(
                doc! {"tx.blockNumber": &unpadded_block_number},
                UpdateModifications::Document(doc! {"$set": {"tx.blockNumber": &padded_block_number}}),
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .expect("Failed to update block number");

        // Same issue as the transactions, we need to update the block number to the padded version once added
        // to the database.
        let header_collection = database.collection::<StoredHeader>();
        let filter = EthDatabaseFilterBuilder::<filter::Header>::default().with_block_number(block_number).build();
        database.update_one(StoredHeader { header }, filter, true).await.expect("Failed to update header in database");
        header_collection
            .update_one(
                doc! {"header.number": unpadded_block_number},
                UpdateModifications::Document(doc! {"$set": {"header.number": padded_block_number}}),
            )
            .with_options(UpdateOptions::builder().upsert(true).build())
            .await
            .expect("Failed to update block number");
    }

    /// Retrieves the first stored transaction
    pub fn first_transaction(&self) -> Option<Transaction> {
        self.transactions.first().map(Into::into)
    }

    /// Retrieves the current block number
    pub fn block_number(&self) -> u64 {
        self.headers.iter().map(|header| header.number.unwrap_or_default()).max().unwrap_or_default()
    }

    /// Retrieves the most recent stored transaction based on block number
    pub fn most_recent_transaction(&self) -> Option<Transaction> {
        self.transactions
            .iter()
            .max_by_key(|stored_transaction| stored_transaction.block_number.unwrap_or_default())
            .map(Into::into)
    }

    /// Retrieves the stored header by hash
    pub fn header_by_hash(&self, hash: B256) -> Option<Header> {
        self.headers.iter().find_map(|stored_header| {
            if stored_header.hash == Some(hash) {
                Some(stored_header.header.clone())
            } else {
                None
            }
        })
    }

    pub fn logs_with_min_topics(&self, min_topics: usize) -> Vec<Log> {
        self.logs.iter().filter(|stored_log| stored_log.topics().len() >= min_topics).map(Into::into).collect()
    }

    pub fn logs_by_address(&self, addresses: &[Address]) -> Vec<Log> {
        self.logs
            .iter()
            .filter(|stored_log| addresses.iter().any(|addr| *addr == stored_log.address()))
            .map(Into::into)
            .collect()
    }

    pub fn logs_by_block_number(&self, block_number: u64) -> Vec<Log> {
        self.logs
            .iter()
            .filter(|stored_log| stored_log.block_number.unwrap_or_default() == block_number)
            .map(Into::into)
            .collect()
    }

    pub fn logs_by_block_range(&self, block_range: std::ops::Range<u64>) -> Vec<Log> {
        self.logs
            .iter()
            .filter(|stored_log| {
                let block_number = stored_log.block_number.unwrap_or_default();
                block_range.contains(&block_number)
            })
            .map(Into::into)
            .collect()
    }

    pub fn logs_by_block_hash(&self, block_hash: B256) -> Vec<Log> {
        self.logs
            .iter()
            .filter(|stored_log| stored_log.block_hash.unwrap_or_default() == block_hash)
            .map(Into::into)
            .collect()
    }

    pub fn all_logs(&self) -> Vec<Log> {
        self.logs.iter().map(Into::into).collect()
    }
}
