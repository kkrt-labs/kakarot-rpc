use async_trait::async_trait;
use kakarot_rpc::eth_provider::{database::EthDatabase, provider::EthProviderResult};
use mongodb::bson::Document;
use serde::de::DeserializeOwned;
pub struct MockDatabase;

#[async_trait]
impl EthDatabase for MockDatabase {
    /// Get a list of documents from a collection
    async fn get<T, F, D>(&self, collection: &str, filter: F, project: D) -> EthProviderResult<Vec<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        D: Into<Option<Document>> + Send,
    {
        Ok(vec![])
    }

    /// Get a single document from a collection
    async fn get_one<T, F, S>(&self, collection: &str, filter: F, sort: S) -> EthProviderResult<Option<T>>
    where
        T: DeserializeOwned + Unpin + Send + Sync,
        F: Into<Option<Document>> + Send,
        S: Into<Option<Document>> + Send,
    {
        Ok(None)
    }

    /// Count the number of documents in a collection matching the filter
    async fn count<D>(&self, collection: &str, filter: D) -> EthProviderResult<u64>
    where
        D: Into<Option<Document>> + Send,
    {
        Ok(1)
    }
}
