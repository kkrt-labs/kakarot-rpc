use crate::eth_provider::constant::U64_PADDING;
use crate::eth_provider::database::types::{
    header::StoredHeader, receipt::StoredTransactionReceipt, transaction::StoredTransaction,
};
use crate::eth_provider::database::Database;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use reth_primitives::{Address, TxType, B256, U128, U256, U64};
use serde::{Serialize, Serializer};
use std::ops::Range;
use std::str::FromStr;
use testcontainers::{
    clients::{self, Cli},
    core::WaitFor,
    Container, GenericImage,
};
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
use {
    arbitrary::Arbitrary, mongodb::bson, reth_primitives::U8, reth_rpc_types::Transaction, std::collections::HashMap,
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
pub const RANDOM_BYTES_SIZE: usize = 100024;

/// Enumeration of collections in the database.
#[derive(Eq, Hash, PartialEq, Clone)]
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
        }
    }
}

/// Struct representing a data generator for MongoDB.
#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
pub struct MongoFuzzer {
    /// Documents to insert into each collection.
    documents: HashMap<CollectionDB, Vec<StoredData>>,
    /// Connection to the MongoDB database.
    mongodb: Database,
    /// Random bytes size.
    rnd_bytes_size: usize,
}

#[cfg(any(test, feature = "arbitrary", feature = "testing"))]
impl MongoFuzzer {
    /// Creates a new instance of `MongoFuzzer` with a bytes size used for generating random data.
    pub async fn new(rnd_bytes_size: usize) -> Self {
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

        Self { documents: Default::default(), mongodb, rnd_bytes_size }
    }

    /// Obtains an immutable reference to the documents HashMap.
    pub fn documents(&self) -> &HashMap<CollectionDB, Vec<StoredData>> {
        &self.documents
    }

    /// Finalizes the data generation and returns the MongoDB database.
    pub async fn finalize(&self) -> Database {
        self.update_collection(CollectionDB::Headers).await;
        self.update_collection(CollectionDB::Transactions).await;
        self.update_collection(CollectionDB::Receipts).await;

        self.mongodb.clone()
    }

    /// Mocks a database with the given number of transactions.
    pub async fn mock_database(rnd_bytes_size: usize, n_transactions: usize) -> Database {
        let mut mongo_fuzzer = Self::new(rnd_bytes_size).await;
        mongo_fuzzer.add_random_transactions(n_transactions).expect("Failed to add documents");
        mongo_fuzzer.finalize().await
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
            let mut header = StoredHeader::arbitrary(&mut unstructured).unwrap();

            header.header.number = Some(U256::from(i as u64));

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

    /// Adds a transaction to the collections of transactions, receipts, and headers.
    fn add_transaction_to_collections(&mut self, transaction: StoredTransaction) {
        let receipt = self.generate_transaction_receipt(&transaction.tx);
        let header = self.generate_transaction_header(&transaction.tx);

        self.documents.entry(CollectionDB::Transactions).or_default().push(StoredData::StoredTransaction(transaction));
        self.documents.entry(CollectionDB::Receipts).or_default().push(StoredData::StoredTransactionReceipt(receipt));
        self.documents.entry(CollectionDB::Headers).or_default().push(StoredData::StoredHeader(header));
    }

    /// Generates a transaction receipt based on the given transaction.
    fn generate_transaction_receipt(&self, transaction: &Transaction) -> StoredTransactionReceipt {
        let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let mut receipt = StoredTransactionReceipt::arbitrary(&mut unstructured).unwrap();

        receipt.receipt.transaction_hash = Some(transaction.hash);
        receipt.receipt.transaction_index = U64::from(transaction.transaction_index.unwrap_or_default());
        receipt.receipt.from = transaction.from;
        receipt.receipt.to = transaction.to;
        receipt.receipt.block_number = transaction.block_number;
        receipt.receipt.block_hash = transaction.block_hash;
        receipt.receipt.transaction_type = U8::from(transaction.transaction_type.unwrap_or_default());
        receipt
    }

    /// Generates a block header based on the given transaction.
    fn generate_transaction_header(&self, transaction: &Transaction) -> StoredHeader {
        let bytes: Vec<u8> = (0..self.rnd_bytes_size).map(|_| rand::random()).collect();
        let mut unstructured = arbitrary::Unstructured::new(&bytes);
        let mut header = StoredHeader::arbitrary(&mut unstructured).unwrap();

        header.header.hash = transaction.block_hash;
        header.header.number = transaction.block_number;
        header
    }

    /// Updates multiple documents in the specified collection.
    async fn update_collection(&self, collection: CollectionDB) {
        let (doc, value, collection_name, updates, block_number) = match collection {
            CollectionDB::Headers => {
                let updates = self.documents.get(&CollectionDB::Headers);
                ("header", "number", "headers", updates, "number")
            }
            CollectionDB::Transactions => {
                let updates = self.documents.get(&CollectionDB::Transactions);
                ("tx", "hash", "transactions", updates, "blockNumber")
            }
            CollectionDB::Receipts => {
                let updates = self.documents.get(&CollectionDB::Receipts);
                ("receipt", "transactionHash", "receipts", updates, "blockNumber")
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
                        block_number: Some(U256::from(BLOCK_NUMBER)),
                        transaction_index: Some(U256::ZERO),
                        from: *RECOVERED_EIP1599_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(U128::from(10)),
                        gas: U256::from(100),
                        max_fee_per_gas: Some(U128::from(10)),
                        max_priority_fee_per_gas: Some(U128::from(1)),
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            v: *TEST_SIG_V,
                            y_parity: Some(reth_rpc_types::Parity(true)),
                        }),
                        chain_id: Some(U64::from(1)),
                        access_list: Some(Default::default()),
                        transaction_type: Some(U64::from(Into::<u8>::into(TxType::Eip1559))),
                        ..Default::default()
                    },
                },
                TxType::Legacy => StoredTransaction {
                    tx: reth_rpc_types::Transaction {
                        hash: *LEGACY_TX_HASH,
                        block_hash: Some(*BLOCK_HASH),
                        block_number: Some(U256::from(BLOCK_NUMBER)),
                        transaction_index: Some(U256::ZERO),
                        from: *RECOVERED_LEGACY_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(U128::from(10)),
                        gas: U256::from(100),
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            // EIP-155 legacy transaction: v = {0,1} + CHAIN_ID * 2 + 35
                            v: CHAIN_ID.saturating_mul(U256::from(2)).saturating_add(U256::from(35)),
                            y_parity: Default::default(),
                        }),
                        chain_id: Some(U64::from(1)),
                        blob_versioned_hashes: Default::default(),
                        transaction_type: Some(U64::from(Into::<u8>::into(TxType::Legacy))),
                        ..Default::default()
                    },
                },
                TxType::Eip2930 => StoredTransaction {
                    tx: reth_rpc_types::Transaction {
                        hash: *EIP2930_TX_HASH,
                        block_hash: Some(*BLOCK_HASH),
                        block_number: Some(U256::from(BLOCK_NUMBER)),
                        transaction_index: Some(U256::ZERO),
                        from: *RECOVERED_EIP2930_TX_ADDRESS,
                        to: Some(Address::ZERO),
                        gas_price: Some(U128::from(10)),
                        gas: U256::from(100),
                        signature: Some(reth_rpc_types::Signature {
                            r: *TEST_SIG_R,
                            s: *TEST_SIG_S,
                            v: *TEST_SIG_V,
                            y_parity: Some(reth_rpc_types::Parity(true)),
                        }),
                        chain_id: Some(U64::from(1)),
                        access_list: Some(Default::default()),
                        transaction_type: Some(U64::from(Into::<u8>::into(TxType::Eip2930))),
                        ..Default::default()
                    },
                },
                _ => unimplemented!(),
            });
        }

        Ok(StoredTransaction::arbitrary(&mut arbitrary::Unstructured::new(&{
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
        // Mocks a database with 100 transactions, receipts and headers.
        let database = MongoFuzzer::mock_database(RANDOM_BYTES_SIZE, 100).await;

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
