use eyre::Result;
use futures::TryStreamExt;
use mongodb::{
    bson::Document,
    options::{FindOneOptions, FindOptions},
    Database,
};
use serde::de::DeserializeOwned;

use super::error::DatabaseError;

pub struct DbClient {
    database: Database,
}

impl DbClient {
    pub fn new(database: Database) -> Self {
        Self { database }
    }

    pub async fn find_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        sort: impl Into<Option<Document>>,
    ) -> Result<T, DatabaseError> {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection = self.database.collection::<T>(collection);
        let result = collection.find_one(filter, find_one_option).await?.ok_or(DatabaseError::ValueNotFound)?;
        Ok(result)
    }

    pub async fn find_all<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> Result<Vec<T>, DatabaseError> {
        let find_options = FindOptions::builder().projection(project).build();
        let collection = self.database.collection::<T>(collection);
        let result = collection.find(filter, find_options).await?.try_collect().await?;
        Ok(result)
    }
}
