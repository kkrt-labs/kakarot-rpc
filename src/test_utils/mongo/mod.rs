use crate::providers::eth_provider::{
    constant::U64_HEX_STRING_LEN,
    database::{
        types::{
            header::StoredHeader, log::StoredLog, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
        },
        CollectionName, Database,
    },
};
use arbitrary::Arbitrary;
use mongodb::{
    bson::{self, doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use reth_primitives::{TxType, B256, U256};
use reth_rpc_types::Transaction;
use serde::Serialize;
use std::{ops::Range, sync::LazyLock};
use strum::{EnumIter, IntoEnumIterator};
use testcontainers::{
    core::{IntoContainerPort, WaitFor},
    runners::AsyncRunner,
    ContainerAsync, Image,
};

pub static CHAIN_ID: LazyLock<U256> = LazyLock::new(|| U256::from(1));

pub const RANDOM_BYTES_SIZE: usize = 100_024;

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

/// Struct representing a data generator for `MongoDB`.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
#[derive(Debug)]
pub struct MongoFuzzer {
    /// Stored headers to insert into the headers collection.
    pub headers: Vec<StoredHeader>,
    /// Stored transactions to insert into the transactions collection.
    pub transactions: Vec<StoredTransaction>,
    /// Stored transaction receipts to insert into the receipts collection.
    pub receipts: Vec<StoredTransactionReceipt>,
    /// Stored logs to insert into the logs collection.
    pub logs: Vec<StoredLog>,
    /// Connection to the [`MongoDB`] database.
    mongodb: Database,
    /// Random bytes size.
    rnd_bytes_size: usize,
    /// Container
    pub container: ContainerAsync<MongoImage>,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl MongoFuzzer {
    /// Asynchronously creates a new instance of `MongoFuzzer`.
    pub async fn new(rnd_bytes_size: usize) -> Self {
        let container = MongoImage.start().await.expect("Failed to start MongoDB container");
        let host_ip = container.get_host().await.expect("Failed to get host IP");
        let port = container.get_host_port_ipv4(27017.tcp()).await.expect("Failed to get host port");
        let url = format!("mongodb://{host_ip}:{port}/");

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

        Self {
            headers: vec![],
            transactions: vec![],
            receipts: vec![],
            logs: vec![],
            mongodb,
            rnd_bytes_size,
            container,
        }
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

        self.headers.push(header);
        Ok(())
    }

    // /// Adds a hardcoded block header range to the collection of headers.
    // pub fn add_hardcoded_block_header_range(&mut self, range: Range<usize>) -> Result<(), Box<dyn std::error::Error>> {
    //     for i in range {
    //         let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
    //         let mut unstructured = arbitrary::Unstructured::new(&bytes);
    //         let mut header = StoredHeader::arbitrary_with_optional_fields(&mut unstructured).unwrap();

    //         header.header.number = Some(i as u64);

    //         self.headers.push(header);
    //     }
    //     Ok(())
    // }

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

            self.logs.push(StoredLog { log });
        }
        Ok(())
    }

    /// Gets the highest block number in the transactions collection.
    pub fn max_block_number(&self) -> u64 {
        self.headers.iter().filter_map(|header| header.number).max().unwrap_or_default()
    }

    /// Adds random transactions to the collection of transactions.
    pub fn add_random_transactions(&mut self, n_transactions: usize) -> Result<(), Box<dyn std::error::Error>> {
        for _ in 0..n_transactions {
            // Build a transaction using the random byte size.
            let transaction = StoredTransaction::arbitrary_with_optional_fields(&mut arbitrary::Unstructured::new(
                &(0..self.rnd_bytes_size).map(|_| rand::random::<u8>()).collect::<Vec<_>>(),
            ))?;

            // Generate a receipt for the transaction.
            let receipt = self.generate_transaction_receipt(&transaction.tx);

            // Convert the receipt into a vector of logs and append them to the existing logs collection.
            self.logs.append(&mut Vec::from(receipt.clone()));

            // Generate a header for the transaction and add it to the headers collection.
            self.headers.push(self.generate_transaction_header(&transaction.tx));

            // Add the transaction to the transactions collection.
            self.transactions.push(transaction);

            // Add the receipt to the receipts collection.
            self.receipts.push(receipt);
        }
        Ok(())
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

    /// Updates the collection with the given collection type.
    async fn update_collection(&self, collection: CollectionDB) {
        match collection {
            CollectionDB::Headers => {
                let collection = self.mongodb.inner().collection(StoredHeader::collection_name());
                self.update_documents(&collection, &self.headers, "header", "number", "number").await;
            }
            CollectionDB::Transactions => {
                let collection = self.mongodb.inner().collection(StoredTransaction::collection_name());
                self.update_documents(&collection, &self.transactions, "tx", "hash", "blockNumber").await;
            }
            CollectionDB::Receipts => {
                let collection = self.mongodb.inner().collection(StoredTransactionReceipt::collection_name());
                self.update_documents(&collection, &self.receipts, "receipt", "transactionHash", "blockNumber").await;
            }
            CollectionDB::Logs => {
                let collection = self.mongodb.inner().collection(StoredLog::collection_name());
                self.update_documents(&collection, &self.logs, "log", "transactionHash", "blockNumber").await;
            }
        }
    }

    /// Updates the documents in the collection with the given documents.
    async fn update_documents<T>(
        &self,
        collection: &Collection<Document>,
        documents: &[T],
        doc: &str,
        value: &str,
        block_number: &str,
    ) where
        T: Serialize,
    {
        let key = [doc, value].join(".");
        let block_key = [doc, block_number].join(".");

        for document in documents {
            // Serialize the StoredData into BSON
            let serialized_data = bson::to_document(document).expect("Failed to serialize StoredData");

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::eth_provider::database::types::{
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

        // Transactions should not be empty.
        assert!(!receipts.is_empty());

        // Transactions should not be empty.
        assert!(!transactions.is_empty());

        // Iterates through transactions and receipts in parallel.
        for (transaction, receipt) in transactions.iter().zip(receipts.iter()) {
            // Asserts equality between transaction block hash and receipt block hash.
            assert_eq!(transaction.block_hash, receipt.receipt.block_hash);

            // Asserts equality between transaction block number and receipt block number.
            assert_eq!(transaction.block_number, receipt.receipt.block_number);

            // Asserts equality between transaction hash and receipt transaction hash.
            assert_eq!(transaction.hash, receipt.receipt.transaction_hash);

            // Asserts equality between transaction index and receipt transaction index.
            assert_eq!(transaction.transaction_index, receipt.receipt.transaction_index);

            // Asserts equality between transaction sender and receipt sender.
            assert_eq!(transaction.from, receipt.receipt.from);

            // Asserts equality between transaction recipient and receipt recipient.
            assert_eq!(transaction.to, receipt.receipt.to);

            // Asserts equality between transaction type and receipt type.
            assert_eq!(transaction.transaction_type.unwrap(), Into::<u8>::into(receipt.receipt.transaction_type()));
        }

        // Drop the inner MongoDB database.
        database.inner().drop().await.unwrap();
    }
}
