use std::str::FromStr;

use crate::eth_provider::database::Database;
use lazy_static::lazy_static;
use mongodb::{
    bson::{doc, Document},
    options::{DatabaseOptions, ReadConcern, UpdateModifications, UpdateOptions, WriteConcern},
    Client, Collection,
};
use reth_primitives::{constants::EMPTY_ROOT_HASH, B256, U128, U256, U64};
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
    pub static ref EIP1599_TX_HASH: B256 = B256::from(U256::from(0x1559));
    pub static ref EIP2930_TX_HASH: B256 = B256::from(U256::from(0x2930));
    pub static ref LEGACY_TX_HASH: B256 = B256::from(U256::from(0x6666));

    pub static ref TEST_SIG_R: U256 = U256::from_str("0x1ae9d63d9152a0f628cc5c843c9d0edc6cb705b027d12d30b871365d7d9c8ed5").unwrap();
    pub static ref TEST_SIG_S: U256 = U256::from_str("0x0d9fa834b490259ad6aa62a49d926053ca1b52acbb59a5e1cf8ecabd65304606").unwrap();
    pub static ref TEST_SIG_V: U256 = U256::from(1);


}

pub const BLOCK_NUMBER: u64 = 0x1234;

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
    let empty_root_hash = format!("{:064x}", EMPTY_ROOT_HASH);
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
                "hash": &hash_256_zero,
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
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "hash": &hash_256_zero,
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
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "hash": &hash_256_zero,
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
                "withdrawalsRoot": &empty_root_hash,
            }},
            doc! {"header": doc! {
                "hash": &hash_256_zero,
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
                "withdrawalsRoot": &empty_root_hash,
            }},
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
                "number": format!("0x{:064x}", BLOCK_NUMBER),
                "gasLimit": &one,
                "gasUsed": &one,
                "timestamp": &hash_256_zero,
                "extraData": "0x",
                "mixHash": &hash_256_zero,
                "baseFeePerGas": &one,
                "withdrawalsRoot": &empty_root_hash,
            }},
        ],
    )
    .await;

    let gas_price_ten = format!("0x{:032x}", U128::from(10));
    let gas_hundred = format!("0x{:064x}", U256::from(100));
    let max_fee_per_gas_ten = format!("0x{:032x}", U128::from(10));
    let max_priority_fee_per_gas_ten = format!("0x{:032x}", U128::from(1));
    let chain_id = format!("0x{:064x}", U256::from(1));
    let tx_eip1559 = format!("0x{:016x}", U64::from(2));
    let tx_eip2930 = format!("0x{:016x}", U64::from(1));
    let tx_legacy = format!("0x{:016x}", U64::from(0));

    let r = format!("0x{:064x}", *TEST_SIG_R);
    let s = format!("0x{:064x}", *TEST_SIG_S);
    let v = format!("0x{:064x}", *TEST_SIG_V);

    update_many(
        "tx".to_string(),
        "hash".to_string(),
        mongodb.collection("transactions"),
        vec![
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *EIP1599_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "to": &address_zero,
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "maxFeePerGas": &max_fee_per_gas_ten,
                "maxPriorityFeePerGas": &max_priority_fee_per_gas_ten,
                "type": &tx_eip1559,
                "chainId": &chain_id,
                "input": "0x",
                "v": &v,
                "r": &r,
                "s": &s,
                "yParity": "0x1",
            }},
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *EIP2930_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "to": &address_zero,
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "type": &tx_eip2930,
                "chainId": &chain_id,
                "input": "0x",
                "v": &v,
                "r": &r,
                "s": &s,
                "yParity": "0x1",
            }},
            doc! {"tx": doc! {
                "hash": format!("0x{:064x}", *LEGACY_TX_HASH),
                "nonce": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "transactionIndex": &zero,
                "from": &address_zero,
                "to": &address_zero,
                "value": &zero,
                "gas": &gas_hundred,
                "gasPrice": &gas_price_ten,
                "type": &tx_legacy,
                "chainId": &chain_id,
                "input": "0x",
                "v": &v,
                "r": &r,
                "s": &s,
            }},
        ],
    )
    .await;

    update_many(
        "receipt".to_string(),
        "transactionHash".to_string(),
        mongodb.collection("receipts"),
        vec![
            doc! {"receipt": doc! {
                "transactionHash": &zero,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs":Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
            }},
            doc! {"receipt": doc! {
                "transactionHash": &one,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs": Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
            }},
            doc! {"receipt": doc! {
                "transactionHash": &two,
                "transactionIndex": &zero,
                "blockHash": format!("0x{:064x}", *BLOCK_HASH),
                "blockNumber": format!("0x{:064x}", BLOCK_NUMBER),
                "from": &address_zero,
                "to": &address_zero,
                "cumulativeGasUsed": &zero,
                "effectiveGasPrice": &zero,
                "gasUsed": &zero,
                "contractAddress": None::<String>,
                "logs": Vec::<Document>::new(),
                "logsBloom": &bloom_zero,
                "type": &zero,
                "status": &zero,
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
