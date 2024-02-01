use crate::eth_provider::database::Database;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use reth_primitives::{B256, U256};
use testcontainers::{
    clients::{self, Cli},
    core::WaitFor,
    Container, GenericImage,
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

    pub static ref BLOCK_HASH: B256 = B256::from(U256::from(0x1234));
    pub static ref BLOCK_NUMBER: u64 = 0x1234;
}

pub async fn mock_database() -> Database {
    let port = CONTAINER.get_host_port_ipv4(27017);

    let mongo_client = Client::with_uri_str(format!("mongodb://root:root@localhost:{}", port))
        .await
        .expect("Failed to init mongo Client");

    let mongodb = mongo_client.database_with_options(
        "kakarot",
        DatabaseOptions::builder().read_concern(ReadConcern::MAJORITY).write_concern(WriteConcern::MAJORITY).build(),
    );

    // Insert one document to create collection
    let hash_256_zero = format!("0x{:064x}", 0);
    let address_zero = format!("0x{:040x}", 0);
    let bloom_zero = format!("0x{:0512x}", 0);

    let zero = format!("0x{:064x}", 0);
    let one = format!("0x{:064x}", 1);
    let two = format!("0x{:064x}", 2);
    let three = format!("0x{:064x}", 3);

    update_many(
        "header".to_string(),
        "number".to_string(),
        mongodb.collection("headers"),
        vec![
            doc! {"header": doc! {
                "hash": format!("0x{:064x}", *BLOCK_HASH),
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &hash_256_zero,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
            }},
            doc! {"header": doc! {
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &one,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
            }},
            doc! {"header": doc! {
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &two,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
            }},
            doc! {"header": doc! {
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": &three,
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
            }},
            doc! {"header": doc! {
                "parentHash": &hash_256_zero,
                "sha3Uncles": &hash_256_zero,
                "miner": &address_zero,
                "stateRoot": &hash_256_zero,
                "transactionsRoot": &hash_256_zero,
                "receiptsRoot": &hash_256_zero,
                "logsBloom": &bloom_zero,
                "difficulty": &hash_256_zero,
                "number": format!("0x{:064x}", *BLOCK_NUMBER),
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
            }},
        ],
    )
    .await;

    update_many(
        "tx".to_string(),
        "hash".to_string(),
        mongodb.collection("transactions"),
        vec![
            doc! {"tx": doc! {
                "hash": &zero,
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", *BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "value": &zero,
                "gas": &zero,
                "input": "0x",
            }},
            doc! {"tx": doc! {
                "hash": &one,
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", *BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "value": &zero,
                "gas": &zero,
                "input": "0x",
            }},
            doc! {"tx": doc! {
                "hash": &two,
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", *BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "value": &zero,
                "gas": &zero,
                "input": "0x",
            }},
        ],
    )
    .await;

    Database::new(mongodb)
}

async fn update_many(doc: String, value: String, collection: Collection<Document>, updates: Vec<Document>) {
    let key = [doc.as_str(), value.as_str()].join(".");
    for u in updates {
        collection
            .update_one(
                doc! {&key: u.get_document(&doc).unwrap().get_str(&value).unwrap()},
                UpdateModifications::Document(doc! {"$set": u}),
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .expect("Failed to insert documents");
    }
}

#[cfg(test)]
mod tests {
    use crate::eth_provider::database::types::header::StoredHeader;

    use super::*;

    #[tokio::test]
    async fn test_mongo_connection() {
        let database = mock_database().await;

        let _ = database.get_one::<StoredHeader>("headers", None, None).await.unwrap();
    }
}
