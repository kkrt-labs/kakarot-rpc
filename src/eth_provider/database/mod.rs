pub mod types;

use async_trait::async_trait;
use futures::TryStreamExt;
use mongodb::{
    bson::Document,
    options::{FindOneOptions, FindOptions},
    Database as MongoDatabase,
};
use serde::de::DeserializeOwned;

use super::provider::EthProviderResult;

#[async_trait]
pub trait EthDatabase {
    /// Get a list of documents from a collection
    async fn get<T, F, P>(&self, collection: &str, filter: F, project: P) -> EthProviderResult<Vec<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        P: Into<Option<Document>> + Send;

    /// Get a single document from a collection
    async fn get_one<T, F, S>(&self, collection: &str, filter: F, sort: S) -> EthProviderResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        S: Into<Option<Document>> + Send;

    /// Count the number of documents in a collection matching the filter
    async fn count<D>(&self, collection: &str, filter: D) -> EthProviderResult<u64>
    where
        D: Into<Option<Document>> + Send;
}

/// Wrapper around a MongoDB database
pub struct Database(MongoDatabase);

impl Database {
    pub fn new(database: MongoDatabase) -> Self {
        Self(database)
    }
}

#[async_trait]
impl EthDatabase for Database {
    /// Get a list of documents from a collection
    async fn get<T, F, D>(&self, collection: &str, filter: F, project: D) -> EthProviderResult<Vec<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        D: Into<Option<Document>> + Send,
    {
        let find_options = FindOptions::builder().projection(project).build();
        let collection = self.0.collection::<T>(collection);
        let result = collection.find(filter, find_options).await?.try_collect().await?;
        Ok(result)
    }

    /// Get a single document from a collection
    async fn get_one<T, F, S>(&self, collection: &str, filter: F, sort: S) -> EthProviderResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        S: Into<Option<Document>> + Send,
    {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection = self.0.collection::<T>(collection);
        let result = collection.find_one(filter, find_one_option).await?;
        Ok(result)
    }

    /// Count the number of documents in a collection matching the filter
    async fn count<D>(&self, collection: &str, filter: D) -> EthProviderResult<u64>
    where
        D: Into<Option<Document>> + Send,
    {
        let collection = self.0.collection::<Document>(collection);
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }
}
