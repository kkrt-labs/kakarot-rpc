use crate::eth_provider::constant::U64_HEX_STRING_LEN;
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
use testcontainers::ContainerAsync;
use testcontainers::{core::IntoContainerPort, runners::AsyncRunner};
use testcontainers::{core::WaitFor, Image};

lazy_static! {
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
pub const RANDOM_BYTES_SIZE: usize = 100_024;

pub fn generate_port_number() -> u16 {
    let address = "0.0.0.0:0";
    let socket = std::net::UdpSocket::bind(address).expect("Cannot bind to socket");
    let local_addr = socket.local_addr().expect("Cannot get local address");
    local_addr.port()
}

#[derive(Default, Debug)]
pub struct MongoImage;

impl Image for MongoImage {
    fn name(&self) -> &str {
        "mongo"
    }

    fn tag(&self) -> &str {
        "6.0.13"
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::Nothing]
    }
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

/// Type alias for the different types of stored data associated with each [`CollectionDB`].
#[derive(Eq, PartialEq, Clone, Debug)]
pub enum StoredData {
    /// Represents a stored header associated with a [`CollectionDB`].
    StoredHeader(StoredHeader),
    /// Represents a stored transaction associated with a [`CollectionDB`].
    StoredTransaction(StoredTransaction),
    /// Represents a stored transaction receipt associated with a [`CollectionDB`].
    StoredTransactionReceipt(StoredTransactionReceipt),
    /// Represents a stored log associated with a [`CollectionDB`].
    StoredLog(StoredLog),
}

impl StoredData {
    /// Extracts the stored header if it exists, otherwise returns None.
    pub const fn extract_stored_header(&self) -> Option<&StoredHeader> {
        match self {
            Self::StoredHeader(header) => Some(header),
            _ => None,
        }
    }

    /// Extracts the stored transaction if it exists, otherwise returns None.
    pub const fn extract_stored_transaction(&self) -> Option<&StoredTransaction> {
        match self {
            Self::StoredTransaction(transaction) => Some(transaction),
            _ => None,
        }
    }

    /// Extracts the stored transaction receipt if it exists, otherwise returns None.
    pub const fn extract_stored_transaction_receipt(&self) -> Option<&StoredTransactionReceipt> {
        match self {
            Self::StoredTransactionReceipt(receipt) => Some(receipt),
            _ => None,
        }
    }

    /// Extracts the stored log if it exists, otherwise returns None.
    pub const fn extract_stored_log(&self) -> Option<&StoredLog> {
        match self {
            Self::StoredLog(log) => Some(log),
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
            Self::StoredHeader(header) => header.serialize(serializer),
            Self::StoredTransaction(transaction) => transaction.serialize(serializer),
            Self::StoredTransactionReceipt(receipt) => receipt.serialize(serializer),
            Self::StoredLog(log) => log.serialize(serializer),
        }
    }
}

/// Struct representing a data generator for `MongoDB`.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
#[derive(Debug)]
pub struct MongoFuzzer {
    /// Documents to insert into each collection.
    documents: HashMap<CollectionDB, Vec<StoredData>>,
    /// Connection to the [`MongoDB`] database.
    mongodb: Database,
    /// Random bytes size.
    rnd_bytes_size: usize,
    /// Port number
    port: u16,
    /// Container
    pub container: ContainerAsync<MongoImage>,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl MongoFuzzer {
    /// Asynchronously creates a new instance of `MongoFuzzer`.
    pub async fn new(rnd_bytes_size: usize) -> Self {
        let node = MongoImage.start().await.expect("Failed to start MongoDB container");
        let host_ip = node.get_host().await.expect("Failed to get host IP");
        let host_port = node.get_host_port_ipv4(27017.tcp()).await.expect("Failed to get host port");
        let url = format!("mongodb://{host_ip}:{host_port}/");

        // Initialize a MongoDB client with the generated port number.
        let mongo_client = Client::with_uri_str(url).await.expect("Failed to init mongo Client");

        // Create a MongoDB database named "kakarot" with specified options.
        let mongodb = mongo_client
            .database_with_options(
                "kakarot",
                DatabaseOptions::builder()
                    .read_concern(ReadConcern::majority())
                    .write_concern(WriteConcern::majority())
                    .build(),
            )
            .into();

        Self { documents: Default::default(), mongodb, rnd_bytes_size, port: host_port, container: node }
    }

    /// Obtains an immutable reference to the documents `HashMap`.
    pub const fn documents(&self) -> &HashMap<CollectionDB, Vec<StoredData>> {
        &self.documents
    }

    /// Get port number
    pub const fn port(&self) -> u16 {
        self.port
    }

    /// Finalizes the data generation and returns the `MongoDB` database.
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

    /// Adds a hardcoded block header with a base fee to the collection of headers.
    pub fn add_hardcoded_block_header_with_base_fee(
        &mut self,
        block_number: u64,
        base_fee: u128,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let mut header = StoredHeader::arbitrary_with_optional_fields(&mut unstructured).unwrap();

        header.header.number = Some(block_number);
        header.header.base_fee_per_gas = Some(base_fee);

        self.documents.entry(CollectionDB::Headers).or_default().push(StoredData::StoredHeader(header));
        Ok(())
    }

    /// Adds a hardcoded block header range to the collection of headers.
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
        self.add_custom_transaction(TransactionBuilder::default().with_tx_type(tx_type.unwrap_or_default()))
    }

    /// Adds random logs to the collection of logs.
    pub fn add_random_logs(&mut self, n_logs: usize) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..n_logs {
            let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
            let mut unstructured = arbitrary::Unstructured::new(&bytes);
            let mut log = StoredLog::arbitrary(&mut unstructured)?.log;

            let topics = log.inner.data.topics_mut_unchecked();
            topics.clear();
            topics.extend([
                B256::arbitrary(&mut unstructured)?,
                B256::arbitrary(&mut unstructured)?,
                B256::arbitrary(&mut unstructured)?,
                B256::arbitrary(&mut unstructured)?,
            ]);

            // Ensure the block number in log <= max block number in the transactions collection.
            log.block_number = Some(log.block_number.unwrap_or_default().min(self.max_block_number()));

            let stored_log = StoredLog { log };

            self.documents.entry(CollectionDB::Logs).or_default().push(StoredData::StoredLog(stored_log));
        }
        Ok(())
    }

    /// Gets the highest block number in the transactions collection.
    pub fn max_block_number(&self) -> u64 {
        self.documents
            .get(&CollectionDB::Headers)
            .unwrap()
            .iter()
            .map(|header| header.extract_stored_header().unwrap().header.number.unwrap_or_default())
            .max()
            .unwrap_or_default()
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

        // Ensure the block number in receipt is equal to the block number in transaction.
        let mut modified_logs = (*receipt.receipt.inner.as_receipt_with_bloom().unwrap()).clone();
        for log in &mut modified_logs.receipt.logs {
            log.block_number = Some(transaction.block_number.unwrap_or_default());
            log.block_hash = transaction.block_hash;
        }

        receipt.receipt.transaction_hash = transaction.hash;
        receipt.receipt.transaction_index = Some(transaction.transaction_index.unwrap_or_default());
        receipt.receipt.from = transaction.from;
        receipt.receipt.to = transaction.to;
        receipt.receipt.block_number = transaction.block_number;
        receipt.receipt.block_hash = transaction.block_hash;
        receipt.receipt.inner = match transaction.transaction_type.unwrap_or_default().try_into() {
            Ok(TxType::Legacy) => reth_rpc_types::ReceiptEnvelope::Legacy(modified_logs),
            Ok(TxType::Eip2930) => reth_rpc_types::ReceiptEnvelope::Eip2930(modified_logs),
            Ok(TxType::Eip1559) => reth_rpc_types::ReceiptEnvelope::Eip1559(modified_logs),
            Ok(TxType::Eip4844) => reth_rpc_types::ReceiptEnvelope::Eip4844(modified_logs),
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
                    )
                    .with_options(UpdateOptions::builder().upsert(true).build())
                    .await
                    .expect("Failed to insert documents");

                let number = serialized_data.get_document(doc).unwrap().get_str(block_number).unwrap();
                let padded_number = format!("0x{:0>width$}", &number[2..], width = U64_HEX_STRING_LEN);

                // Update the document by padding the block number to U64_HEX_STRING_LEN value.
                collection
                    .update_one(
                        doc! {&block_key: &number},
                        UpdateModifications::Document(doc! {"$set": {&block_key: padded_number}}),
                    )
                    .with_options(UpdateOptions::builder().upsert(true).build())
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
    #[must_use]
    pub const fn with_tx_type(mut self, tx_type: TxType) -> Self {
        self.tx_type = Some(tx_type);
        self
    }

    /// Builds the transaction based on the specified values.
    fn build(self, rnd_bytes_size: usize) -> Result<StoredTransaction, Box<dyn std::error::Error>> {
        Ok(match self.tx_type {
            Some(tx_type) => StoredTransaction::mock_tx_with_type(tx_type),
            None => StoredTransaction::arbitrary_with_optional_fields(&mut arbitrary::Unstructured::new(
                &(0..rnd_bytes_size).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
            ))?,
        })
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

        // Mocks a database with 100 transactions, receipts and headers.
        let database = mongo_fuzzer.mock_database(100).await;

        // Retrieves stored headers from the database.
        let _ = database.get_all::<StoredHeader>().await.unwrap();

        // Retrieves stored transactions from the database.
        let transactions = database.get_all::<StoredTransaction>().await.unwrap();

        // Retrieves stored receipts from the database.
        let receipts = database.get_all::<StoredTransactionReceipt>().await.unwrap();

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
        database.inner().drop().await.unwrap();
    }
}
