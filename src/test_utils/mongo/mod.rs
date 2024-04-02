use std::str::FromStr;

use crate::eth_provider::database::Database;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use rand::Rng;
use reth_primitives::{constants::EMPTY_ROOT_HASH, Address, B256, U128, U256, U64};
use std::collections::HashMap;
use testcontainers::{
    clients::{self, Cli},
    core::WaitFor,
    Container, GenericImage,
};

lazy_static! {
    static ref DOCKER_CLI: Cli = clients::Cli::default();
    static ref IMAGE: GenericImage = GenericImage::new("mongo", "6.0.13")
        .with_wait_for(WaitFor::message_on_stdout("server is ready"))
        .with_env_var("MONGO_INITDB_DATABASE", "kakarot")
        .with_env_var("MONGO_INITDB_ROOT_USERNAME", "root")
        .with_env_var("MONGO_INITDB_ROOT_PASSWORD", "root")
        .with_exposed_port(27017);
    // The container is made static to avoid dropping it before the tests are finished.
    static ref CONTAINER: Container<'static, GenericImage> = DOCKER_CLI.run(IMAGE.clone());

    pub static ref CHAIN_ID: U256 = U256::from(1);

    pub static ref BLOCK_HASH: B256 = B256::from(U256::from(0x1234));
    pub static ref EIP1599_TX_HASH: B256 = B256::from(U256::from_str("0xc92a4e464caa049999cb2073cc4d8586bebb42b518115f631710b2597155c962").unwrap());
    pub static ref EIP2930_TX_HASH: B256 = B256::from(U256::from_str("0x972ba18c780c31bade31873d6f076a3be4e6d314776e2ad50a30eda861acab79").unwrap());
    pub static ref LEGACY_TX_HASH: B256 = B256::from(U256::from_str("0x38c7e066854c56932100b896320a37adbab32713cca46d1e06307fe5d6062b7d").unwrap());

    pub static ref TEST_SIG_R: U256 = U256::from_str("0x1ae9d63d9152a0f628cc5c843c9d0edc6cb705b027d12d30b871365d7d9c8ed5").unwrap();
    pub static ref TEST_SIG_S: U256 = U256::from_str("0x0d9fa834b490259ad6aa62a49d926053ca1b52acbb59a5e1cf8ecabd65304606").unwrap();
    pub static ref TEST_SIG_V: U256 = U256::from(1);
    // Given constant r, s, v and transaction fields, we can derive the `Transaction.from` field "a posteriori"
    // ⚠️ If the transaction fields change, the below addresses should be updated accordingly ⚠️
    // Recovered address from the above R, S, V, with EIP1559 transaction
    pub static ref RECOVERED_EIP1599_TX_ADDRESS: Address = Address::from_str("0x520666a744f86a09c2a794b8d56501c109afba2d").unwrap();
    // Recovered address from the above R, S, V, with EIP2930 transaction
    pub static ref RECOVERED_EIP2930_TX_ADDRESS: Address = Address::from_str("0x753925d9bbd7682e4b77f102c47d24ee0580aa8d").unwrap();
    // Recovered address from the above R, S, V, with Legacy transaction
    pub static ref RECOVERED_LEGACY_TX_ADDRESS: Address = Address::from_str("0x05ac0c7c5930a6f9003a709042dbb136e98220f2").unwrap();


}

pub const BLOCK_NUMBER: u64 = 0x1234;

pub async fn mock_database() -> Database {
    let port = CONTAINER.get_host_port_ipv4(27017);

    let mongo_client = Client::with_uri_str(format!("mongodb://root:root@localhost:{}", port))
        .await
        .expect("Failed to init mongo Client");

    let mongodb = mongo_client.database_with_options(
        "kakarot",
        DatabaseOptions::builder().read_concern(ReadConcern::MAJORITY).write_concern(WriteConcern::MAJORITY).build(),
    );

    // Insert one document to create collection
    let empty_root_hash = format!("{:064x}", EMPTY_ROOT_HASH);
    let hash_256_zero = format!("0x{:064x}", 0);
    let address_zero = format!("0x{:040x}", 0);
    let bloom_zero = format!("0x{:0512x}", 0);
    let nonce_zero = format!("0x{:016x}", 0);

    let zero = format!("0x{:064x}", 0);
    let one = format!("0x{:064x}", 1);
    let two = format!("0x{:064x}", 2);
    let three = format!("0x{:064x}", 3);

    update_many(
        "header".to_string(),
        "number".to_string(),
        mongodb.collection("headers"),
        vec![
            doc! {"header": doc! {
                "nonce": &nonce_zero,
                "hash": &hash_256_zero,
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &hash_256_zero,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "nonce": &nonce_zero,
                "hash": &hash_256_zero,
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &one,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "nonce": &nonce_zero,
                "hash": &hash_256_zero,
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &two,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "nonce": &nonce_zero,
                "hash": &hash_256_zero,
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &three,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "nonce": &nonce_zero,
                "hash": format!("0x{:064x}", *BLOCK_HASH),
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": format!("0x{:064x}", BLOCK_NUMBER),
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
                "withdrawalsRoot": &empty_root_hash,
            }},
        ],
    )
    .await;

    let gas_price_ten = format!("0x{:032x}", U128::from(10));
    let gas_hundred = format!("0x{:064x}", U256::from(100));
    let max_fee_per_gas_ten = format!("0x{:032x}", U128::from(10));
    let max_priority_fee_per_gas_ten = format!("0x{:032x}", U128::from(1));
    let chain_id = format!("0x{:064x}", *CHAIN_ID);
    let tx_eip1559 = format!("0x{:016x}", U64::from(2));
    let tx_eip2930 = format!("0x{:016x}", U64::from(1));
    let tx_legacy = format!("0x{:016x}", U64::from(0));

    let r = format!("0x{:064x}", *TEST_SIG_R);
    let s = format!("0x{:064x}", *TEST_SIG_S);
    let v = format!("0x{:064x}", *TEST_SIG_V);

    update_many(
        "tx".to_string(),
        "hash".to_string(),
        mongodb.collection("transactions"),
        vec![
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *EIP1599_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &format!("0x{:040x}", *RECOVERED_EIP1599_TX_ADDRESS),
                "to": &address_zero,
                "accessList": [],
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "maxFeePerGas": &max_fee_per_gas_ten,
                "maxPriorityFeePerGas": &max_priority_fee_per_gas_ten,
                "type": &tx_eip1559,
                "chainId": &chain_id,
                "input": "0x",
                "v": &v,
                "r": &r,
                "s": &s,
                "yParity": "0x1",
            }},
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *EIP2930_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": format!("0x{:040x}", *RECOVERED_EIP2930_TX_ADDRESS),
                "accessList": [],
                "to": &address_zero,
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "type": &tx_eip2930,
                "chainId": &chain_id,
                "input": "0x",
                "v": &v,
                "r": &r,
                "s": &s,
                "yParity": "0x1",
            }},
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *LEGACY_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &format!("0x{:040x}", *RECOVERED_LEGACY_TX_ADDRESS),
                "to": &address_zero,
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "type": &tx_legacy,
                "chainId": &chain_id,
                "input": "0x",
                // EIP-155 legacy transaction: v = {0,1} + CHAIN_ID * 2 + 35
                "v": &format!("0x{:064x}", CHAIN_ID.saturating_mul(U256::from(2)).saturating_add(U256::from(35))),
                "r": &r,
                "s": &s,
            }},
        ],
    )
    .await;

    update_many(
        "receipt".to_string(),
        "transactionHash".to_string(),
        mongodb.collection("receipts"),
        vec![
            doc! {"receipt": doc! {
                "transactionHash": &zero,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs":Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
            }},
            doc! {"receipt": doc! {
                "transactionHash": &one,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs": Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
            }},
            doc! {"receipt": doc! {
                "transactionHash": &two,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs": Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
            }},
        ],
    )
    .await;

    Database::new(mongodb)
}

async fn update_many(doc: String, value: String, collection: Collection<Document>, updates: Vec<Document>) {
    let key = [doc.as_str(), value.as_str()].join(".");
    for u in updates {
        collection
            .update_one(
                doc! {&key: u.get_document(&doc).unwrap().get_str(&value).unwrap()},
                UpdateModifications::Document(doc! {"$set": u}),
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .expect("Failed to insert documents");
    }
}

/// Enumeration of collections in the database.
#[derive(Eq, Hash, PartialEq)]
pub enum CollectionDB {
    /// Collection of block headers.
    Headers,
    /// Collection of transactions.
    Transactions,
    /// Collection of transaction receipts.
    Receipts,
}

/// Struct representing a data generator for MongoDB.
pub struct MongoFuzzer<R: Rng + Clone> {
    /// Random number generator.
    rng: R,
    /// Documents to insert into each collection.
    documents: HashMap<CollectionDB, Vec<Document>>,
    /// Connection to the MongoDB database.
    mongodb: Database,
}

impl<R: Rng + Clone> MongoFuzzer<R> {
    /// Creates a new instance of `MongoFuzzer` with the specified random number generator.
    pub async fn new(rng: R) -> Self {
        let port = CONTAINER.get_host_port_ipv4(27017);

        let mongo_client = Client::with_uri_str(format!("mongodb://root:root@localhost:{}", port))
            .await
            .expect("Failed to init mongo Client");

        let mongodb = mongo_client
            .database_with_options(
                "kakarot",
                DatabaseOptions::builder()
                    .read_concern(ReadConcern::MAJORITY)
                    .write_concern(WriteConcern::MAJORITY)
                    .build(),
            )
            .into();

        Self { rng, documents: Default::default(), mongodb }
    }

    /// Finalizes the data generation and returns the MongoDB database.
    pub async fn finalize(&self) -> Database {
        self.update_many(CollectionDB::Headers).await;
        self.update_many(CollectionDB::Transactions).await;
        self.update_many(CollectionDB::Receipts).await;

        self.mongodb.clone()
    }

    /// Mocks a database with the given number of headers and transactions.
    pub async fn mock_database(rng: R, n_headers: usize, n_transactions: usize) -> Database {
        let mut mongo_fuzzer = Self::new(rng).await;
        mongo_fuzzer.add_headers(n_headers);
        mongo_fuzzer.add_transactions(n_transactions);
        mongo_fuzzer.finalize().await
    }

    /// Generates a document representing a block header.
    pub fn header_document(&mut self) -> Document {
        doc! {
            "nonce": self.generate_random_hex_string(16),
            "hash": self.generate_random_hex_string(64),
            "parentHash": self.generate_random_hex_string(64),
            "sha3Uncles": self.generate_random_hex_string(64),
            "miner": self.generate_random_hex_string(40),
            "stateRoot": self.generate_random_hex_string(64),
            "transactionsRoot": self.generate_random_hex_string(64),
            "receiptsRoot": self.generate_random_hex_string(64),
            "logsBloom": self.generate_random_hex_string(512),
            "difficulty": self.generate_random_hex_string(64),
            "number": self.generate_random_hex_string(16),
            "gasLimit": self.generate_random_hex_string(16),
            "gasUsed": self.generate_random_hex_string(16),
            "timestamp": self.generate_random_hex_string(16),
            "extraData": self.generate_random_hex_string(64),
            "mixHash": self.generate_random_hex_string(64),
            "withdrawalsRoot": self.generate_random_hex_string(64),
        }
    }

    /// Generates documents representing a transaction and its receipt.
    pub fn transaction_documents(&mut self) -> (Document, Document) {
        let tx_hash = self.generate_random_hex_string(64);
        let tx_index = self.generate_random_hex_string(16);
        let from = self.generate_random_hex_string(40);
        let to = self.generate_random_hex_string(40);
        let block_number = self.generate_random_hex_string(16);
        let block_hash = self.generate_random_hex_string(64);
        let tx_type = self.generate_random_hex_string_to(3);

        let access_list: Vec<_> = (0..=self.rng.gen_range(0..=10))
            .map(|_| {
                let storage_keys =
                    (0..=self.rng.gen_range(0..=10)).map(|_| self.generate_random_hex_string(64)).collect::<Vec<_>>();
                doc! {
                    "address": self.generate_random_hex_string(40),
                    "storageKeys": storage_keys
                }
            })
            .collect();

        let tx = doc! {
            "hash": tx_hash.clone(),
            "nonce": self.generate_random_hex_string(16),
            "blockHash": block_hash.clone(),
            "blockNumber": block_number.clone(),
            "transactionIndex": tx_index.clone(),
            "from": from.clone(),
            "to": to.clone(),
            "accessList": access_list.clone(),
            "value": self.generate_random_hex_string(64),
            "gas": self.generate_random_hex_string(32),
            "gasPrice": self.generate_random_hex_string(32),
            "maxFeePerGas": self.generate_random_hex_string(32),
            "maxPriorityFeePerGas": self.generate_random_hex_string(32),
            "type": tx_type.clone(),
            "chainId": self.generate_random_hex_string(16),
            "input": self.generate_random_hex_string(64),
            "v": self.generate_random_hex_string(64),
            "r": self.generate_random_hex_string(64),
            "s": self.generate_random_hex_string(64),
            "yParity": self.generate_random_hex_string_to(1),
        };

        let logs: Vec<_> = (0..=self.rng.gen_range(0..=10))
            .map(|_| {
                let topics =
                    (0..=self.rng.gen_range(0..=10)).map(|_| self.generate_random_hex_string(64)).collect::<Vec<_>>();
                doc! {
                    "address": self.generate_random_hex_string(40),
                    "topics": topics,
                    "data": self.generate_random_hex_string(64),
                }
            })
            .collect();

        let receipt = doc! {
            "transactionHash": tx_hash.clone(),
            "transactionIndex": tx_index.clone(),
            "blockHash": block_hash.clone(),
            "blockNumber": block_number.clone(),
            "from": from.clone(),
            "to": to.clone(),
            "cumulativeGasUsed": self.generate_random_hex_string(16),
            "effectiveGasPrice": self.generate_random_hex_string(32),
            "gasUsed": self.generate_random_hex_string(64),
            "contractAddress": self.generate_random_hex_string(40),
            "logs": logs.clone(),
            "logsBloom": self.generate_random_hex_string(512),
            "type": tx_type.clone(),
            "status": self.generate_random_hex_string_to(1),
        };

        (tx, receipt)
    }

    /// Adds a document to the specified collection.
    pub fn add_document(&mut self, collection: CollectionDB) {
        match collection {
            CollectionDB::Transactions | CollectionDB::Receipts => {
                let (tx, receipt) = self.transaction_documents();
                self.documents.entry(CollectionDB::Transactions).or_insert_with(Vec::new).push(tx);
                self.documents.entry(CollectionDB::Receipts).or_insert_with(Vec::new).push(receipt);
            }
            CollectionDB::Headers => {
                let header = self.header_document();
                self.documents.entry(CollectionDB::Headers).or_insert_with(Vec::new).push(header);
            }
        }
    }

    /// Adds multiple transactions to the database.
    pub fn add_transactions(&mut self, n_transactions: usize) {
        for _ in 0..n_transactions {
            self.add_document(CollectionDB::Transactions);
        }
    }

    /// Adds multiple headers to the database.
    pub fn add_headers(&mut self, n_headers: usize) {
        for _ in 0..n_headers {
            self.add_document(CollectionDB::Headers);
        }
    }

    /// Generates a random hexadecimal string of the specified length.
    fn generate_random_hex_string(&mut self, length: usize) -> String {
        const HEX_CHARS: &[u8] = b"0123456789abcdef";
        let chars: String = (0..length)
            .map(|_| {
                let idx = self.rng.gen_range(0..HEX_CHARS.len());
                HEX_CHARS[idx] as char
            })
            .collect();
        format!("0x{}", chars)
    }

    /// Generates a random hexadecimal string up to the specified value.
    fn generate_random_hex_string_to(&mut self, to: usize) -> String {
        let random_number = self.rng.gen_range(0..=to);
        format!("0x{:x}", random_number)
    }

    /// Updates multiple documents in the specified collection.
    async fn update_many(&self, collection: CollectionDB) {
        let (doc, value, collection_name, updates) = match collection {
            CollectionDB::Headers => {
                let updates = self.documents.get(&CollectionDB::Headers);
                ("header", "number", "headers", updates)
            }
            CollectionDB::Transactions => {
                let updates = self.documents.get(&CollectionDB::Transactions);
                ("tx", "hash", "transactions", updates)
            }
            CollectionDB::Receipts => {
                let updates = self.documents.get(&CollectionDB::Receipts);
                ("receipt", "transactionHash", "receipts", updates)
            }
        };

        let collection: Collection<Document> = self.mongodb.collection(collection_name);
        let key = [doc, value].join(".");

        if let Some(updates) = updates {
            for u in updates {
                let u = doc! {doc: u};
                collection
                    .update_one(
                        doc! {&key: u.get_document(&doc).unwrap().get_str(&value).unwrap()},
                        UpdateModifications::Document(doc! {"$set": u}),
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await
                    .expect("Failed to insert documents");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth_provider::database::types::{
        header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
    };

    #[tokio::test]
    async fn test_mongo_connection() {
        let database = mock_database().await;
        let _ = database.get_one::<StoredHeader>("headers", None, None).await.unwrap();
    }

    #[tokio::test]
    async fn test_mongo_fuzzer() {
        // Mocks a database with 10 headers and 10 transactions.
        let database = MongoFuzzer::mock_database(rand::thread_rng(), 10, 10).await;

        // Retrieves stored headers from the database.
        let _ = database.get::<StoredHeader>("headers", None, None).await.unwrap();

        // Retrieves stored transactions from the database.
        let transactions = database.get::<StoredTransaction>("transactions", None, None).await.unwrap();

        // Retrieves stored receipts from the database.
        let receipts = database.get::<StoredTransactionReceipt>("receipts", None, None).await.unwrap();

        // Iterates through transactions and receipts in parallel.
        for (transaction, receipt) in transactions.iter().zip(receipts.iter()) {
            // Asserts equality between transaction block hash and receipt block hash.
            assert_eq!(transaction.tx.block_hash, receipt.receipt.block_hash);

            // Asserts equality between transaction block number and receipt block number.
            assert_eq!(transaction.tx.block_number.unwrap(), receipt.receipt.block_number.unwrap());

            // Asserts equality between transaction hash and receipt transaction hash.
            assert_eq!(transaction.tx.hash, receipt.receipt.transaction_hash.unwrap());

            // Asserts equality between transaction index and receipt transaction index.
            assert_eq!(transaction.tx.transaction_index.unwrap(), U256::from(receipt.receipt.transaction_index));

            // Asserts equality between transaction sender and receipt sender.
            assert_eq!(transaction.tx.from, receipt.receipt.from);

            // Asserts equality between transaction recipient and receipt recipient.
            assert_eq!(transaction.tx.to.unwrap(), receipt.receipt.to.unwrap());

            // Asserts equality between transaction type and receipt type.
            assert_eq!(transaction.tx.transaction_type.unwrap(), U64::from(receipt.receipt.transaction_type));
        }
    }
}
