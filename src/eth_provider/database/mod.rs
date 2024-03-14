pub mod types;

use futures::TryStreamExt;
use mongodb::{
    bson::Document,
    options::{FindOneOptions, FindOptions},
    Database as MongoDatabase,
};
use serde::de::DeserializeOwned;

use super::provider::EthProviderResult;

/// Wrapper around a MongoDB database
pub struct Database(MongoDatabase);

impl Database {
    pub const fn new(database: MongoDatabase) -> Self {
        Self(database)
    }
}

impl Database {
    /// Get a list of documents from a collection
    pub async fn get<T>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> EthProviderResult<Vec<T>>
    where
        T: DeserializeOwned,
    {
        let find_options = FindOptions::builder().projection(project).build();
        let collection = self.0.collection::<T>(collection);
        let result = collection.find(filter, find_options).await?.try_collect().await?;
        Ok(result)
    }

    /// Get a single document from a collection
    pub async fn get_one<T>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        sort: impl Into<Option<Document>>,
    ) -> EthProviderResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
    {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection = self.0.collection::<T>(collection);
        let result = collection.find_one(filter, find_one_option).await?;
        Ok(result)
    }

    /// Count the number of documents in a collection matching the filter
    pub async fn count(&self, collection: &str, filter: impl Into<Option<Document>>) -> EthProviderResult<u64> {
        let collection = self.0.collection::<Document>(collection);
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }
}
