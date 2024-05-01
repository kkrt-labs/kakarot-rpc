pub mod types;

use super::error::KakarotError;
use crate::eth_provider::database::types::{
    header::StoredHeader,
    log::StoredLog,
    receipt::StoredTransactionReceipt,
    transaction::{StoredPendingTransaction, StoredTransaction, StoredTransactionHash},
};
use futures::TryStreamExt;
use itertools::Itertools;
use mongodb::{
    bson::{doc, Document},
    options::{FindOneOptions, FindOptions, UpdateModifications, UpdateOptions},
    Collection, Database as MongoDatabase,
};
use serde::{de::DeserializeOwned, Serialize};

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

    /// Returns a collection from the database.
    pub fn collection<T>(&self) -> Collection<T>
    where
        T: CollectionName,
    {
        self.0.collection::<T>(T::collection_name())
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
        Ok(self.collection::<T>().find(filter, find_options).await?.try_collect().await?)
    }

    /// Retrieves documents from a collection and converts them into another type.
    ///
    /// Returns a vector of documents of type `D` if successful, or an error.
    pub async fn get_and_map_to<D, T>(
        &self,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<D>>
    where
        T: DeserializeOwned + CollectionName,
        D: From<T>,
    {
        let stored_data: Vec<T> = self.get(filter, project).await?;
        Ok(stored_data.into_iter().map_into().collect())
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
        Ok(self.collection::<T>().find_one(filter, find_one_option).await?)
    }

    /// Get a single document from aggregated collections
    pub async fn get_one_aggregate<T>(&self, pipeline: impl IntoIterator<Item = Document>) -> DatabaseResult<Option<T>>
    where
        T: DeserializeOwned + CollectionName,
    {
        let mut cursor = self.collection::<T>().aggregate(pipeline, None).await?;

        Ok(cursor.try_next().await?.map(|doc| mongodb::bson::de::from_document(doc)).transpose()?)
    }

    /// Update a single document in a collection
    pub async fn update_one<T>(&self, doc: T, filter: impl Into<Document>, upsert: bool) -> DatabaseResult<()>
    where
        T: Serialize + CollectionName,
    {
        let doc = mongodb::bson::to_document(&doc).map_err(mongodb::error::Error::custom)?;

        self.collection::<T>()
            .update_one(
                filter.into(),
                UpdateModifications::Document(doc! {"$set": doc}),
                UpdateOptions::builder().upsert(upsert).build(),
            )
            .await?;

        Ok(())
    }

    /// Delete a single document from a collection
    pub async fn delete_one<T>(&self, filter: impl Into<Document>) -> DatabaseResult<()>
    where
        T: CollectionName,
    {
        self.collection::<T>().delete_one(filter.into(), None).await?;
        Ok(())
    }

    /// Count the number of documents in a collection matching the filter
    pub async fn count<T>(&self, filter: impl Into<Option<Document>>) -> DatabaseResult<u64>
    where
        T: CollectionName,
    {
        Ok(self.collection::<T>().count_documents(filter, None).await?)
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
