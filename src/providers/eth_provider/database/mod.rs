pub mod ethereum;
pub mod filter;
pub mod state;
pub mod types;

use super::error::KakarotError;
use crate::providers::eth_provider::database::types::{
    header::StoredHeader,
    log::StoredLog,
    receipt::StoredTransactionReceipt,
    transaction::{StoredEthStarknetTransactionHash, StoredTransaction},
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

/// Struct for encapsulating find options for `MongoDB` queries.
#[derive(Clone, Debug, Default)]
pub struct FindOpts(FindOptions);

impl FindOpts {
    /// Sets the limit for the number of documents to retrieve.
    #[must_use]
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.0.limit = Some(i64::try_from(limit).unwrap_or(i64::MAX));
        self
    }

    /// Sets the projection for the documents to retrieve.
    #[must_use]
    pub fn with_projection(mut self, projection: Document) -> Self {
        self.0.projection = Some(projection);
        self
    }

    /// Builds and returns the `FindOptions`.
    pub fn build(self) -> FindOptions {
        self.0
    }
}

/// Wrapper around a `MongoDB` database
#[derive(Clone, Debug)]
pub struct Database(MongoDatabase);

impl Database {
    pub const fn new(database: MongoDatabase) -> Self {
        Self(database)
    }

    /// Get a reference to the inner `MongoDatabase`
    pub const fn inner(&self) -> &MongoDatabase {
        &self.0
    }

    /// Get a mutable reference to the inner `MongoDatabase`
    pub fn inner_mut(&mut self) -> &mut MongoDatabase {
        &mut self.0
    }

    /// Returns a collection from the database.
    pub fn collection<T>(&self) -> Collection<T>
    where
        T: CollectionName + Sync + Send,
    {
        self.0.collection::<T>(T::collection_name())
    }

    /// Get a list of documents from a collection
    pub async fn get<T>(
        &self,
        filter: impl Into<Option<Document>>,
        find_options: impl Into<Option<FindOpts>>,
    ) -> DatabaseResult<Vec<T>>
    where
        T: DeserializeOwned + CollectionName + Sync + Send,
    {
        let find_options = find_options.into();
        Ok(self
            .collection::<T>()
            .find(Into::<Option<Document>>::into(filter).unwrap_or_default())
            .with_options(find_options.unwrap_or_default().build())
            .await?
            .try_collect()
            .await?)
    }

    /// Get all documents from a collection
    pub async fn get_all<T>(&self) -> DatabaseResult<Vec<T>>
    where
        T: DeserializeOwned + CollectionName + Sync + Send,
    {
        let find_options = FindOpts::default().build();

        Ok(self.collection::<T>().find(Default::default()).with_options(find_options).await?.try_collect().await?)
    }

    /// Retrieves documents from a collection and converts them into another type.
    ///
    /// Returns a vector of documents of type `D` if successful, or an error.
    pub async fn get_and_map_to<D, T>(
        &self,
        filter: impl Into<Option<Document>>,
        find_options: Option<FindOpts>,
    ) -> DatabaseResult<Vec<D>>
    where
        T: DeserializeOwned + CollectionName + Sync + Send,
        D: From<T>,
    {
        let stored_data: Vec<T> = self.get(filter, find_options).await?;
        Ok(stored_data.into_iter().map_into().collect())
    }

    /// Retrieves all documents from a collection and converts them into another type.
    ///
    /// Returns a vector of documents of type `D` if successful, or an error.
    pub async fn get_all_and_map_to<D, T>(&self) -> DatabaseResult<Vec<D>>
    where
        T: DeserializeOwned + CollectionName + Sync + Send,
        D: From<T>,
    {
        let stored_data: Vec<T> = self.get_all().await?;
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
        let find_one_options = FindOneOptions::builder().sort(sort).build();
        Ok(self
            .collection::<T>()
            .find_one(Into::<Option<Document>>::into(filter).unwrap_or_default())
            .with_options(find_one_options)
            .await?)
    }

    /// Get the first document from a collection
    pub async fn get_first<T>(&self) -> DatabaseResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync + CollectionName,
    {
        Ok(self.collection::<T>().find_one(Default::default()).await?)
    }

    /// Get a single document from aggregated collections
    pub async fn get_one_aggregate<T>(&self, pipeline: impl IntoIterator<Item = Document>) -> DatabaseResult<Option<T>>
    where
        T: DeserializeOwned + CollectionName + Sync + Send,
    {
        let mut cursor = self.collection::<T>().aggregate(pipeline).await?;

        Ok(cursor.try_next().await?.map(|doc| mongodb::bson::de::from_document(doc)).transpose()?)
    }

    /// Update a single document in a collection
    pub async fn update_one<T>(&self, doc: T, filter: impl Into<Document>, upsert: bool) -> DatabaseResult<()>
    where
        T: Serialize + CollectionName + Sync + Send,
    {
        let doc = mongodb::bson::to_document(&doc).map_err(mongodb::error::Error::custom)?;
        let update_options = UpdateOptions::builder().upsert(upsert).build();

        self.collection::<T>()
            .update_one(filter.into(), UpdateModifications::Document(doc! {"$set": doc}))
            .with_options(update_options)
            .await?;

        Ok(())
    }

    /// Delete a single document from a collection
    pub async fn delete_one<T>(&self, filter: impl Into<Document>) -> DatabaseResult<()>
    where
        T: CollectionName + Sync + Send,
    {
        self.collection::<T>().delete_one(filter.into()).await?;
        Ok(())
    }

    /// Count the number of documents in a collection matching the filter
    pub async fn count<T>(&self, filter: Document) -> DatabaseResult<u64>
    where
        T: CollectionName + Sync + Send,
    {
        Ok(self.collection::<T>().count_documents(filter).await?)
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

/// Implement [`CollectionName`] for [`StoredEthStarknetTransactionHash`]
impl CollectionName for StoredEthStarknetTransactionHash {
    fn collection_name() -> &'static str {
        "transaction_hashes"
    }
}
