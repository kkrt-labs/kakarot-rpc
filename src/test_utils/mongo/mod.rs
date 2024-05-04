use crate::eth_provider::constant::U64_PADDING;
use crate::eth_provider::database::types::{
    header::StoredHeader, log::StoredLog, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
};
use crate::eth_provider::database::{CollectionName, Database};
use arbitrary::Arbitrary;
use lazy_static::lazy_static;
use mongodb::{
    bson::{self, doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use reth_primitives::{Address, TxType, B256, U256};
use reth_rpc_types::Transaction;
use serde::{Serialize, Serializer};
use std::collections::HashMap;
use std::ops::Range;
use std::str::FromStr;
use strum::{EnumIter, IntoEnumIterator};
use testcontainers::clients::{self, Cli};
use testcontainers::{GenericImage, RunnableImage};

lazy_static! {
    pub static ref DOCKER_CLI: Cli = clients::Cli::default();
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
pub const RANDOM_BYTES_SIZE: usize = 100024;

pub fn generate_port_number() -> u16 {
    let address = "0.0.0.0:0";
    let socket = std::net::UdpSocket::bind(address).expect("Cannot bind to socket");
    let local_addr = socket.local_addr().expect("Cannot get local address");
    local_addr.port()
}

/// Enumeration of collections in the database.
#[derive(Eq, Hash, PartialEq, Clone, Debug, EnumIter)]
pub enum CollectionDB {
    /// Collection of block headers.
    Headers,
    /// Collection of transactions.
    Transactions,
    /// Collection of transaction receipts.
    Receipts,
    /// Collection of logs.
    Logs,
}

/// Type alias for the different types of stored data associated with each CollectionDB.
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum StoredData {
    /// Represents a stored header associated with a CollectionDB.
    StoredHeader(StoredHeader),
    /// Represents a stored transaction associated with a CollectionDB.
    StoredTransaction(StoredTransaction),
    /// Represents a stored transaction receipt associated with a CollectionDB.
    StoredTransactionReceipt(StoredTransactionReceipt),
    /// Represents a stored log associated with a CollectionDB.
    StoredLog(StoredLog),
}

impl StoredData {
    /// Extracts the stored header if it exists, otherwise returns None.
    pub fn extract_stored_header(&self) -> Option<&StoredHeader> {
        match self {
            StoredData::StoredHeader(header) => Some(header),
            _ => None,
        }
    }

    /// Extracts the stored transaction if it exists, otherwise returns None.
    pub fn extract_stored_transaction(&self) -> Option<&StoredTransaction> {
        match self {
            StoredData::StoredTransaction(transaction) => Some(transaction),
            _ => None,
        }
    }

    /// Extracts the stored transaction receipt if it exists, otherwise returns None.
    pub fn extract_stored_transaction_receipt(&self) -> Option<&StoredTransactionReceipt> {
        match self {
            StoredData::StoredTransactionReceipt(receipt) => Some(receipt),
            _ => None,
        }
    }
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
            StoredData::StoredLog(log) => log.serialize(serializer),
        }
    }
}

/// Struct representing a data generator for MongoDB.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
#[derive(Debug)]
pub struct MongoFuzzer {
    /// Documents to insert into each collection.
    documents: HashMap<CollectionDB, Vec<StoredData>>,
    /// Connection to the MongoDB database.
    mongodb: Database,
    /// Random bytes size.
    rnd_bytes_size: usize,
    // Port number
    port: u16,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl MongoFuzzer {
    /// Asynchronously creates a new instance of `MongoFuzzer`.
    pub async fn new(rnd_bytes_size: usize) -> Self {
        // Generate a random port number.
        let port = generate_port_number();

        // Initialize a MongoDB client with the generated port number.
        let mongo_client = Client::with_uri_str(format!("mongodb://{}:{}", "0.0.0.0", port))
            .await
            .expect("Failed to init mongo Client");

        // Create a MongoDB database named "kakarot" with specified options.
        let mongodb = mongo_client
            .database_with_options(
                "kakarot",
                DatabaseOptions::builder()
                    .read_concern(ReadConcern::MAJORITY)
                    .write_concern(WriteConcern::MAJORITY)
                    .build(),
            )
            .into();

        Self { documents: Default::default(), mongodb, rnd_bytes_size, port }
    }

    /// Obtains an immutable reference to the documents HashMap.
    pub fn documents(&self) -> &HashMap<CollectionDB, Vec<StoredData>> {
        &self.documents
    }

    /// Get MongoDB image
    pub fn mongo_image(&self) -> RunnableImage<GenericImage> {
        let image = GenericImage::new("mongo".to_string(), "6.0.13".to_string());
        RunnableImage::from(image).with_mapped_port((self.port, 27017))
    }

    /// Get port number
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Finalizes the data generation and returns the MongoDB database.
    pub async fn finalize(&self) -> Database {
        for collection in CollectionDB::iter() {
            self.update_collection(collection).await;
        }

        self.mongodb.clone()
    }

    /// Mocks a database with the given number of transactions.
    pub async fn mock_database(&mut self, n_transactions: usize) -> Database {
        self.add_random_transactions(n_transactions).expect("Failed to add documents");
        self.finalize().await
    }

    /// Adds a transaction to the collection of transactions with custom values.
    pub fn add_custom_transaction(&mut self, builder: TransactionBuilder) -> Result<(), Box<dyn std::error::Error>> {
        let transaction = builder.build(self.rnd_bytes_size)?;
        self.add_transaction_to_collections(transaction);
        Ok(())
    }

    pub fn add_hardcoded_block_header_range(&mut self, range: Range<usize>) -> Result<(), Box<dyn std::error::Error>> {
        for i in range {
            let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
            let mut unstructured = arbitrary::Unstructured::new(&bytes);
            let mut header = StoredHeader::arbitrary_with_optional_fields(&mut unstructured).unwrap();

            header.header.number = Some(i as u64);

            self.documents.entry(CollectionDB::Headers).or_default().push(StoredData::StoredHeader(header));
        }
        Ok(())
    }

    /// Adds a hardcoded transaction to the collection of transactions.
    pub fn add_hardcoded_transaction(&mut self, tx_type: Option<TxType>) -> Result<(), Box<dyn std::error::Error>> {
        let builder = TransactionBuilder::default().with_tx_type(tx_type.unwrap_or_default());
        self.add_custom_transaction(builder)
    }

    /// Adds a hardcoded transaction to the collection of transactions.
    pub fn add_random_transaction(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let builder = TransactionBuilder::default();
        self.add_custom_transaction(builder)
    }

    /// Adds random transactions to the collection of transactions.
    pub fn add_random_transactions(&mut self, n_transactions: usize) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..n_transactions {
            self.add_random_transaction()?;
        }
        Ok(())
    }

    /// Adds a transaction to the collections of transactions, receipts, logs, and headers.
    fn add_transaction_to_collections(&mut self, transaction: StoredTransaction) {
        let receipt = self.generate_transaction_receipt(&transaction.tx);
        let mut logs = Vec::<StoredLog>::from(receipt.clone()).into_iter().map(StoredData::StoredLog).collect();

        let header = self.generate_transaction_header(&transaction.tx);

        self.documents.entry(CollectionDB::Transactions).or_default().push(StoredData::StoredTransaction(transaction));
        self.documents.entry(CollectionDB::Receipts).or_default().push(StoredData::StoredTransactionReceipt(receipt));
        self.documents.entry(CollectionDB::Logs).or_default().append(&mut logs);
        self.documents.entry(CollectionDB::Headers).or_default().push(StoredData::StoredHeader(header));
    }

    /// Generates a transaction receipt based on the given transaction.
    fn generate_transaction_receipt(&self, transaction: &Transaction) -> StoredTransactionReceipt {
        let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let mut receipt = StoredTransactionReceipt::arbitrary(&mut unstructured).unwrap();

        receipt.receipt.transaction_hash = transaction.hash;
        receipt.receipt.transaction_index = Some(transaction.transaction_index.unwrap_or_default());
        receipt.receipt.from = transaction.from;
        receipt.receipt.to = transaction.to;
        receipt.receipt.block_number = transaction.block_number;
        receipt.receipt.block_hash = transaction.block_hash;
        receipt.receipt.inner = match transaction.transaction_type.unwrap_or_default().try_into() {
            Ok(TxType::Legacy) => reth_rpc_types::ReceiptEnvelope::Legacy(
                (*receipt.receipt.inner.as_receipt_with_bloom().unwrap()).clone(),
            ),
            Ok(TxType::Eip2930) => reth_rpc_types::ReceiptEnvelope::Eip2930(
                (*receipt.receipt.inner.as_receipt_with_bloom().unwrap()).clone(),
            ),
            Ok(TxType::Eip1559) => reth_rpc_types::ReceiptEnvelope::Eip1559(
                (*receipt.receipt.inner.as_receipt_with_bloom().unwrap()).clone(),
            ),
            Ok(TxType::Eip4844) => reth_rpc_types::ReceiptEnvelope::Eip4844(
                (*receipt.receipt.inner.as_receipt_with_bloom().unwrap()).clone(),
            ),
            Err(_) => unreachable!(),
        };
        receipt
    }

    /// Generates a block header based on the given transaction.
    fn generate_transaction_header(&self, transaction: &Transaction) -> StoredHeader {
        let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let mut header = StoredHeader::arbitrary_with_optional_fields(&mut unstructured).unwrap();

        header.header.hash = transaction.block_hash;
        header.header.number = transaction.block_number;
        header
    }

    /// Updates multiple documents in the specified collection.
    async fn update_collection(&self, collection: CollectionDB) {
        let (doc, value, collection_name, updates, block_number) = match collection {
            CollectionDB::Headers => {
                let updates = self.documents.get(&CollectionDB::Headers);
                ("header", "number", StoredHeader::collection_name(), updates, "number")
            }
            CollectionDB::Transactions => {
                let updates = self.documents.get(&CollectionDB::Transactions);
                ("tx", "hash", StoredTransaction::collection_name(), updates, "blockNumber")
            }
            CollectionDB::Receipts => {
                let updates = self.documents.get(&CollectionDB::Receipts);
                ("receipt", "transactionHash", StoredTransactionReceipt::collection_name(), updates, "blockNumber")
            }
            CollectionDB::Logs => {
                let updates = self.documents.get(&CollectionDB::Logs);
                ("log", "transactionHash", StoredLog::collection_name(), updates, "blockNumber")
            }
        };

        let collection: Collection<Document> = self.mongodb.inner().collection(collection_name);
        let key = [doc, value].join(".");
        let block_key = [doc, block_number].join(".");

        if let Some(updates) = updates {
            for u in updates {
                // Serialize the StoredData into BSON
                let serialized_data = bson::to_document(u).expect("Failed to serialize StoredData");

                // Insert the document in the collection
                collection
                    .update_one(
                        doc! {&key: serialized_data.get_document(doc).unwrap().get_str(value).unwrap()},
                        UpdateModifications::Document(doc! {"$set": serialized_data.clone()}),
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await
                    .expect("Failed to insert documents");

                let number = serialized_data.get_document(doc).unwrap().get_str(block_number).unwrap();
                let padded_number = format!("0x{:0>width$}", &number[2..], width = U64_PADDING);

                // Update the document by padding the block number to U64_PADDING value.
                collection
                    .update_one(
                        doc! {&block_key: &number},
                        UpdateModifications::Document(doc! {"$set": {&block_key: padded_number}}),
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await
                    .expect("Failed to insert documents");
            }
        }
    }
}

/// Builder for constructing transactions with custom values.
#[derive(Default, Clone, Debug)]
pub struct TransactionBuilder {
    /// The type of transaction to construct.
    tx_type: Option<TxType>,
}

impl TransactionBuilder {
    /// Specifies the type of transaction to build.
    pub fn with_tx_type(mut self, tx_type: TxType) -> Self {
        self.tx_type = Some(tx_type);
        self
    }

    /// Builds the transaction based on the specified values.
    fn build(self, rnd_bytes_size: usize) -> Result<StoredTransaction, Box<dyn std::error::Error>> {
        if let Some(tx_type) = self.tx_type {
            return Ok(match tx_type {
                TxType::Eip1559 => StoredTransaction {
                    tx: reth_rpc_types::Transaction {
                        hash: *EIP1599_TX_HASH,
                        block_hash: Some(*BLOCK_HASH),
                        block_number: Some(BLOCK_NUMBER),
                        transaction_index: Some(0),
                        from: *RECOVERED_EIP1599_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(10),
                        gas: 100,
                        max_fee_per_gas: Some(10),
                        max_priority_fee_per_gas: Some(1),
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            v: *TEST_SIG_V,
                            y_parity: Some(reth_rpc_types::Parity(true)),
                        }),
                        chain_id: Some(1),
                        access_list: Some(Default::default()),
                        transaction_type: Some(TxType::Eip1559.into()),
                        ..Default::default()
                    },
                },
                TxType::Legacy => StoredTransaction {
                    tx: reth_rpc_types::Transaction {
                        hash: *LEGACY_TX_HASH,
                        block_hash: Some(*BLOCK_HASH),
                        block_number: Some(BLOCK_NUMBER),
                        transaction_index: Some(0),
                        from: *RECOVERED_LEGACY_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(10),
                        gas: 100,
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            // EIP-155 legacy transaction: v = {0,1} + CHAIN_ID * 2 + 35
                            v: CHAIN_ID.saturating_mul(U256::from(2)).saturating_add(U256::from(35)),
                            y_parity: Default::default(),
                        }),
                        chain_id: Some(1),
                        blob_versioned_hashes: Default::default(),
                        transaction_type: Some(TxType::Legacy.into()),
                        ..Default::default()
                    },
                },
                TxType::Eip2930 => StoredTransaction {
                    tx: reth_rpc_types::Transaction {
                        hash: *EIP2930_TX_HASH,
                        block_hash: Some(*BLOCK_HASH),
                        block_number: Some(BLOCK_NUMBER),
                        transaction_index: Some(0),
                        from: *RECOVERED_EIP2930_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(10),
                        gas: 100,
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            v: *TEST_SIG_V,
                            y_parity: Some(reth_rpc_types::Parity(true)),
                        }),
                        chain_id: Some(1),
                        access_list: Some(Default::default()),
                        transaction_type: Some(TxType::Eip2930.into()),
                        ..Default::default()
                    },
                },
                _ => unimplemented!(),
            });
        }

        Ok(StoredTransaction::arbitrary_with_optional_fields(&mut arbitrary::Unstructured::new(&{
            (0..rnd_bytes_size).map(|_| rand::random::<u8>()).collect::<Vec<_>>()
        }))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eth_provider::database::types::{
        header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
    };

    #[tokio::test]
    async fn test_mongo_fuzzer() {
        // Generate a MongoDB fuzzer
        let mut mongo_fuzzer = MongoFuzzer::new(RANDOM_BYTES_SIZE).await;

        // Run docker
        let _c = DOCKER_CLI.run(mongo_fuzzer.mongo_image());

        // Mocks a database with 100 transactions, receipts and headers.
        let database = mongo_fuzzer.mock_database(100).await;

        // Retrieves stored headers from the database.
        let _ = database.get::<StoredHeader>(None, None).await.unwrap();

        // Retrieves stored transactions from the database.
        let transactions = database.get::<StoredTransaction>(None, None).await.unwrap();

        // Retrieves stored receipts from the database.
        let receipts = database.get::<StoredTransactionReceipt>(None, None).await.unwrap();

        // Iterates through transactions and receipts in parallel.
        for (transaction, receipt) in transactions.iter().zip(receipts.iter()) {
            // Asserts equality between transaction block hash and receipt block hash.
            assert_eq!(transaction.tx.block_hash, receipt.receipt.block_hash);

            // Asserts equality between transaction block number and receipt block number.
            assert_eq!(transaction.tx.block_number, receipt.receipt.block_number);

            // Asserts equality between transaction hash and receipt transaction hash.
            assert_eq!(transaction.tx.hash, receipt.receipt.transaction_hash);

            // Asserts equality between transaction index and receipt transaction index.
            assert_eq!(transaction.tx.transaction_index, receipt.receipt.transaction_index);

            // Asserts equality between transaction sender and receipt sender.
            assert_eq!(transaction.tx.from, receipt.receipt.from);

            // Asserts equality between transaction recipient and receipt recipient.
            assert_eq!(transaction.tx.to, receipt.receipt.to);

            // Asserts equality between transaction type and receipt type.
            assert_eq!(transaction.tx.transaction_type.unwrap(), Into::<u8>::into(receipt.receipt.transaction_type()));
        }

        // Drop the inner MongoDB database.
        database.inner().drop(None).await.unwrap();
    }
}
