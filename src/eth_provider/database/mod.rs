pub mod types;

use super::error::KakarotError;
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
        collection: &str,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<T>>
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
    ) -> DatabaseResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
    {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection = self.0.collection::<T>(collection);
        let result = collection.find_one(filter, find_one_option).await?;
        Ok(result)
    }

    /// Update a single document in a collection
    pub async fn update_one<T>(
        &self,
        collection: &str,
        query: Document,
        update: impl Into<UpdateModifications>,
        options: impl Into<Option<UpdateOptions>>,
    ) -> DatabaseResult<()>
    where
        T: DeserializeOwned,
    {
        self.0.collection::<T>(collection).update_one(query, update, options.into()).await?;
        Ok(())
    }

    /// Count the number of documents in a collection matching the filter
    pub async fn count(&self, collection: &str, filter: impl Into<Option<Document>>) -> DatabaseResult<u64> {
        let collection = self.0.collection::<Document>(collection);
        let count = collection.count_documents(filter, None).await?;
        Ok(count)
    }
}

impl From<MongoDatabase> for Database {
    fn from(database: MongoDatabase) -> Self {
        Self(database)
    }
}
