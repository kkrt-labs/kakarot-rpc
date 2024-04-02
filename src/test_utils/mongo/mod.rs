use crate::eth_provider::database::types::{
    header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
};
use crate::eth_provider::database::Database;
#[cfg(any(test, feature = "arbitrary"))]
use arbitrary::Arbitrary;
use lazy_static::lazy_static;
#[cfg(any(test, feature = "arbitrary"))]
use mongodb::bson;
use mongodb::{
    bson::{doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
#[cfg(any(test, feature = "arbitrary"))]
use reth_primitives::U8;
use reth_primitives::{constants::EMPTY_ROOT_HASH, Address, B256, U128, U256, U64};
use serde::{Serialize, Serializer};
#[cfg(any(test, feature = "arbitrary"))]
use std::collections::HashMap;
use std::str::FromStr;
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

/// Type alias for the different types of stored data associated with each CollectionDB.
#[derive(Eq, PartialEq, Clone)]
pub enum StoredData {
    /// Represents a stored header associated with a CollectionDB.
    StoredHeader(StoredHeader),
    /// Represents a stored transaction associated with a CollectionDB.
    StoredTransaction(StoredTransaction),
    /// Represents a stored transaction receipt associated with a CollectionDB.
    StoredTransactionReceipt(StoredTransactionReceipt),
}

impl Serialize for StoredData {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            StoredData::StoredHeader(header) => header.serialize(serializer),
            StoredData::StoredTransaction(transaction) => transaction.serialize(serializer),
            StoredData::StoredTransactionReceipt(receipt) => receipt.serialize(serializer),
        }
    }
}

/// Struct representing a data generator for MongoDB.
#[cfg(any(test, feature = "arbitrary"))]
pub struct MongoFuzzer<'a> {
    /// Documents to insert into each collection.
    documents: HashMap<CollectionDB, Vec<StoredData>>,
    /// Connection to the MongoDB database.
    mongodb: Database,
    /// Unstructured data
    u: &'a mut arbitrary::Unstructured<'a>,
}

#[cfg(any(test, feature = "arbitrary"))]
impl<'a> MongoFuzzer<'a> {
    /// Creates a new instance of `MongoFuzzer` with the specified random number generator.
    pub async fn new<'b>(u: &'b mut arbitrary::Unstructured<'a>) -> Self
    where
        'b: 'a,
    {
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

        Self { u, documents: Default::default(), mongodb }
    }

    /// Finalizes the data generation and returns the MongoDB database.
    pub async fn finalize(&self) -> Database {
        self.update_many(CollectionDB::Headers).await;
        self.update_many(CollectionDB::Transactions).await;
        self.update_many(CollectionDB::Receipts).await;

        self.mongodb.clone()
    }

    /// Mocks a database with the given number of headers and transactions.
    pub async fn mock_database<'b>(
        u: &'b mut arbitrary::Unstructured<'a>,
        n_headers: usize,
        n_transactions: usize,
    ) -> Database
    where
        'b: 'a,
    {
        let mut mongo_fuzzer = MongoFuzzer::new(u).await;
        mongo_fuzzer.add_headers(n_headers);
        mongo_fuzzer.add_transactions(n_transactions);
        mongo_fuzzer.finalize().await
    }

    /// Adds a document to the specified collection.
    pub fn add_document(&mut self, collection: CollectionDB) -> Result<(), Box<dyn std::error::Error>> {
        match collection {
            CollectionDB::Transactions | CollectionDB::Receipts => {
                let transaction = StoredTransaction::arbitrary(self.u)?;
                let mut receipt = StoredTransactionReceipt::arbitrary(self.u)?;
                receipt.receipt.transaction_hash = Some(transaction.tx.hash);
                receipt.receipt.transaction_index = U64::from(transaction.tx.transaction_index.unwrap_or_default());
                receipt.receipt.from = transaction.tx.from;
                receipt.receipt.to = transaction.tx.to;
                receipt.receipt.block_number = transaction.tx.block_number;
                receipt.receipt.block_hash = transaction.tx.block_hash;
                receipt.receipt.transaction_type = U8::from(transaction.tx.transaction_type.unwrap_or_default());

                self.documents
                    .entry(CollectionDB::Transactions)
                    .or_default()
                    .push(StoredData::StoredTransaction(transaction));
                self.documents
                    .entry(CollectionDB::Receipts)
                    .or_default()
                    .push(StoredData::StoredTransactionReceipt(receipt));
            }
            CollectionDB::Headers => {
                let header = StoredHeader::arbitrary(self.u)?;
                self.documents.entry(CollectionDB::Headers).or_default().push(StoredData::StoredHeader(header));
            }
        }

        Ok(())
    }

    /// Adds multiple transactions to the database.
    pub fn add_transactions(&mut self, n_transactions: usize) {
        for _ in 0..n_transactions {
            self.add_document(CollectionDB::Transactions).expect("Failed to add transaction");
        }
    }

    /// Adds multiple headers to the database.
    pub fn add_headers(&mut self, n_headers: usize) {
        for _ in 0..n_headers {
            self.add_document(CollectionDB::Headers).expect("Failed to add header");
        }
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

        let collection: Collection<Document> = self.mongodb.inner().collection(collection_name);
        let key = [doc, value].join(".");

        if let Some(updates) = updates {
            for u in updates {
                // Serialize the StoredData into BSON
                let serialized_data = bson::to_document(u).expect("Failed to serialize StoredData");

                collection
                    .update_one(
                        doc! {&key: serialized_data.get_document(doc).unwrap().get_str(value).unwrap()},
                        UpdateModifications::Document(doc! {"$set": serialized_data}),
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
    use rand::Rng;

    #[tokio::test]
    async fn test_mongo_connection() {
        // Create a mock database.
        let database = mock_database().await;

        // Retrieve a single document from the "headers" collection.
        let _ = database.get_one::<StoredHeader>("headers", None, None).await.unwrap();

        // Drop the inner MongoDB database.
        database.inner().drop(None).await.unwrap();
    }

    #[tokio::test]
    async fn test_mongo_fuzzer() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());

        // Mocks a database with 100 headers and 100 transactions.
        let database = MongoFuzzer::mock_database(&mut arbitrary::Unstructured::new(&bytes), 100, 100).await;

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
            assert_eq!(transaction.tx.block_number, receipt.receipt.block_number);

            // Asserts equality between transaction hash and receipt transaction hash.
            assert_eq!(transaction.tx.hash, receipt.receipt.transaction_hash.unwrap());

            // Asserts equality between transaction index and receipt transaction index.
            assert_eq!(transaction.tx.transaction_index.unwrap(), U256::from(receipt.receipt.transaction_index));

            // Asserts equality between transaction sender and receipt sender.
            assert_eq!(transaction.tx.from, receipt.receipt.from);

            // Asserts equality between transaction recipient and receipt recipient.
            assert_eq!(transaction.tx.to, receipt.receipt.to);

            // Asserts equality between transaction type and receipt type.
            assert_eq!(transaction.tx.transaction_type.unwrap(), U64::from(receipt.receipt.transaction_type));
        }

        // Drop the inner MongoDB database.
        database.inner().drop(None).await.unwrap();
    }
}
