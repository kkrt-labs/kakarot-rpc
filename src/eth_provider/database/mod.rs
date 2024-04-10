pub mod types;

use super::error::KakarotError;
use crate::eth_provider::database::types::{
    header::StoredHeader,
    log::StoredLog,
    receipt::StoredTransactionReceipt,
    transaction::{StoredPendingTransaction, StoredTransaction, StoredTransactionHash},
};
use futures::TryStreamExt;
use mongodb::{
    bson::Document,
    options::{FindOneOptions, FindOptions, UpdateModifications, UpdateOptions},
    Database as MongoDatabase,
};
use serde::de::DeserializeOwned;

type DatabaseResult<T> = eyre::Result<T, KakarotError>;

/// Wrapper around a MongoDB database
#[derive(Clone, Debug)]
pub struct Database(MongoDatabase);

impl Database {
    pub const fn new(database: MongoDatabase) -> Self {
        Self(database)
    }

    /// Get a reference to the inner MongoDatabase
    pub fn inner(&self) -> &MongoDatabase {
        &self.0
    }

    /// Get a mutable reference to the inner MongoDatabase
    pub fn inner_mut(&mut self) -> &mut MongoDatabase {
        &mut self.0
    }

    /// Get a list of documents from a collection
    pub async fn get<T>(
        &self,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<T>>
    where
        T: DeserializeOwned + CollectionName,
    {
        let find_options = FindOptions::builder().projection(project).build();
        let collection_name = T::collection_name();
        let collection = self.0.collection::<T>(collection_name);
        let result = collection.find(filter, find_options).await?.try_collect().await?;
        Ok(result)
    }

    /// Get a single document from a collection
    pub async fn get_one<T>(
        &self,
        filter: impl Into<Option<Document>>,
        sort: impl Into<Option<Document>>,
    ) -> DatabaseResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync + CollectionName,
    {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection_name = T::collection_name();
        let collection = self.0.collection::<T>(collection_name);
        let result = collection.find_one(filter, find_one_option).await?;
        Ok(result)
    }

    /// Update a single document in a collection
    pub async fn update_one<T>(
        &self,
        query: Document,
        update: impl Into<UpdateModifications>,
        options: impl Into<Option<UpdateOptions>>,
    ) -> DatabaseResult<()>
    where
        T: DeserializeOwned + CollectionName,
    {
        let collection_name = T::collection_name();
        self.0.collection::<T>(collection_name).update_one(query, update, options.into()).await?;
        Ok(())
    }

    /// Count the number of documents in a collection matching the filter
    pub async fn count<T>(&self, filter: impl Into<Option<Document>>) -> DatabaseResult<u64>
    where
        T: CollectionName,
    {
        let collection_name = T::collection_name();
        let collection = self.0.collection::<Document>(collection_name);
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }
}

impl From<MongoDatabase> for Database {
    fn from(database: MongoDatabase) -> Self {
        Self(database)
    }
}

/// Trait for associating a type with its collection name
pub trait CollectionName {
    /// Returns the name of the collection associated with the type
    fn collection_name() -> &'static str;
}

/// Implement [`CollectionName`] for [`StoredHeader`]
impl CollectionName for StoredHeader {
    fn collection_name() -> &'static str {
        "headers"
    }
}

/// Implement [`CollectionName`] for [`StoredTransaction`]
impl CollectionName for StoredTransaction {
    fn collection_name() -> &'static str {
        "transactions"
    }
}

/// Implement [`CollectionName`] for [`StoredPendingTransaction`]
impl CollectionName for StoredPendingTransaction {
    fn collection_name() -> &'static str {
        "transactions_pending"
    }
}

/// Implement [`CollectionName`] for [`StoredTransactionHash`]
impl CollectionName for StoredTransactionHash {
    fn collection_name() -> &'static str {
        "transactions"
    }
}

/// Implement [`CollectionName`] for [`StoredTransactionReceipt`]
impl CollectionName for StoredTransactionReceipt {
    fn collection_name() -> &'static str {
        "receipts"
    }
}

/// Implement [`CollectionName`] for [`StoredLog`]
impl CollectionName for StoredLog {
    fn collection_name() -> &'static str {
        "logs"
    }
}
