use eyre::Result;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::{FindOneOptions, FindOptions},
    Database as MongoDatabase,
};
use reth_primitives::{H256, U64};
use reth_rpc_types::{Block, BlockTransactions, RichBlock};
use serde::de::DeserializeOwned;

use super::types::header::StoredHeader;
use super::{
    error::DatabaseError,
    types::transaction::{StoredTransactionFull, StoredTransactionHash},
};

type DatabaseResult<T> = Result<T, DatabaseError>;

pub struct EthDatabase {
    database: MongoDatabase,
}

impl EthDatabase {
    pub fn new(database: MongoDatabase) -> Self {
        Self { database }
    }

    async fn get<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        project: impl Into<Option<Document>>,
    ) -> DatabaseResult<Vec<T>> {
        let find_options = FindOptions::builder().projection(project).build();
        let collection = self.database.collection::<T>(collection);
        let result = collection.find(filter, find_options).await?.try_collect().await?;
        Ok(result)
    }

    async fn get_one<T: DeserializeOwned + Unpin + Send + Sync>(
        &self,
        collection: &str,
        filter: impl Into<Option<Document>>,
        sort: impl Into<Option<Document>>,
    ) -> DatabaseResult<T> {
        let find_one_option = FindOneOptions::builder().sort(sort).build();
        let collection = self.database.collection::<T>(collection);
        let result = collection.find_one(filter, find_one_option).await?.ok_or(DatabaseError::ValueNotFound)?;
        Ok(result)
    }

    pub async fn block_number(&self) -> DatabaseResult<U64> {
        let filter = doc! {};
        let sort = doc! { "header.number": -1 };
        let header: StoredHeader = self.get_one("headers", filter, sort).await?;
        let block_number = header.header.number.ok_or(DatabaseError::ValueNotFound)?.as_limbs()[0];
        Ok(block_number.into())
    }

    pub async fn chain_id(&self) -> DatabaseResult<Option<U64>> {
        let tx: StoredTransactionFull = self.get_one("transactions", doc! {}, doc! {"tx.blockNumber": -1}).await?;
        Ok(tx.tx.chain_id)
    }

    pub async fn block_by_hash(&self, hash: H256, full: bool) -> DatabaseResult<Option<RichBlock>> {
        let header =
            self.get_one::<StoredHeader>("headers", doc! {"header.hash": format!("0x{:064x}", hash)}, None).await?;
        let total_difficulty = Some(header.header.difficulty);

        let transactions = if full {
            BlockTransactions::Full(
                self.get::<StoredTransactionFull>(
                    "transactions",
                    doc! {"tx.blockHash": format!("0x{:064x}", hash)},
                    None,
                )
                .await?
                .into_iter()
                .map(|tx| tx.tx)
                .collect(),
            )
        } else {
            BlockTransactions::Hashes(
                self.get::<StoredTransactionHash>(
                    "transactions",
                    doc! {"tx.blockHash": format!("0x{:064x}", hash)},
                    doc! {"tx.blockHash": 1},
                )
                .await?
                .into_iter()
                .map(|tx| tx.tx.into())
                .collect(),
            )
        };

        let block = Block {
            header: header.header,
            transactions,
            total_difficulty,
            uncles: Vec::new(),
            size: None,
            withdrawals: None,
        };

        Ok(Some(block.into()))
    }
}
