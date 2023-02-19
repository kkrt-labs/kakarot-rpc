mod testing_helpers;

#[cfg(test)]
mod tests {
    use crate::testing_helpers::{assert_block, assert_block_header};
    use kakarot_rpc::test_utils::setup_rpc_server;
    use kakarot_rpc_core::{
        client::{
            constants::CHAIN_ID,
            types::{Block, BlockTransactions, Transaction},
        },
        helpers::{felt_option_to_u256, felt_to_u256, starknet_address_to_ethereum_address},
        utils::wiremock_utils::EthJsonRpcResponse,
    };
    use reth_primitives::{Bloom, Bytes, H160, H256, H64, U256, U64};
    use reth_rpc_types::TransactionReceipt;
    use serde_json::json;
    use starknet::core::types::FieldElement;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_block_number_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_blockNumber\", \"params\": [] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();
        let block_number = res.json::<EthJsonRpcResponse<String>>().await.unwrap();
        assert_eq!(
            block_number.result,
            "0x0000000000000000000000000000000000000000000000000000000000004cb8"
        );

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_get_block_by_hash_hydrated_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByHash\", \"params\": [\"0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9\", true] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let block = res.json::<EthJsonRpcResponse<Block>>().await.unwrap();

        let starknet_res = json!({
            "block_hash": "0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9",
            "block_number": 19612,
            "new_root": "0x67cde84ecff30c4ca55cb46df37940df87a94cc416cb893eaa9fb4fb67ec513",
            "parent_hash": "0x137970a5417cf7d35eb4eeb04efe6312166f828eec76342338b0e3797ebf3c1",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1675461581,
        });

        let starknet_block_txs = json!({
            "transactions": [{
            "calldata": [
                "0x1",
                "0x4a3621276a83251b557a8140e915599ae8e7b6207b067ea701635c0d509801e",
                "0x2d4c8ea4c8fb9f571d1f6f9b7692fff8e5ceaf73b1df98e7da8c1109b39ae9a",
                "0x0",
                "0x2",
                "0x2",
                "0x4767b873669406d25dddbf67356e385a14480979e5358a411955d692576aa30",
                "0x1"
            ],
            "max_fee": "0x23b29a4eb4000",
            "nonce": "0x7",
            "sender_address": "0x78ec7936f688d2768c038f54c0f8be71f4e7b6a4ef0ce4a83c96b6a25e225df",
            "signature": [
                "0x3f5acbd7644c45f9559a5f253684bdaee95a02571c384df0855896336cc4e66",
                "0x29710b4b8d5f7a0a3f160c40afb38395c669020de02c439e18dd4894d8402f4"
            ],
            "transaction_hash": "0x1e8741e0a53ada441371400e12879e1f085c3ae39f073e68c04c25dd58e3f8d",
            "type": "INVOKE",
            "version": "0x1"
        }
        ]
        });

        assert_block(
            block.result.clone(),
            starknet_res.to_string(),
            starknet_block_txs.to_string(),
            true,
        );
        assert_block_header(block.result.clone(), starknet_res.to_string());

        // assert_block_transactions(block.result.clone(), starknet_block_txs.to_string());

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_get_block_by_hash_not_hydrated_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByHash\", \"params\": [\"0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e\", false] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let block = res.json::<EthJsonRpcResponse<Block>>().await.unwrap();

        // Header data
        let starknet_block_hash = FieldElement::from_str(
            "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
        )
        .unwrap();
        assert_eq!(
            block.result.header.hash,
            Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
        );
        assert_eq!(block.result.header.number, Some(U256::from(19639)));

        let starknet_parent_hash = FieldElement::from_str(
            "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
        )
        .unwrap();
        let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
        assert_eq!(block.result.header.parent_hash, parent_hash);
        assert_eq!(block.result.header.uncles_hash, parent_hash);

        let starknet_sequencer = FieldElement::from_str(
            "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
        )
        .unwrap();
        let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
        assert_eq!(block.result.header.author, sequencer);
        assert_eq!(block.result.header.miner, sequencer);

        let starknet_new_root = FieldElement::from_str(
            "0x5549eb2dffae1d468fff16454cb2f44cdeea63ca79f56730304b170faecdd3b",
        )
        .unwrap();
        let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
        assert_eq!(block.result.header.state_root, state_root);

        assert_eq!(block.result.header.transactions_root, H256::zero());
        assert_eq!(block.result.header.receipts_root, H256::zero());

        assert_eq!(block.result.header.extra_data, Bytes::from(b"0x00"));
        assert_eq!(block.result.header.logs_bloom, Bloom::default());
        assert_eq!(block.result.header.timestamp, U256::from(1675496282));

        //TODO: update when real data fetched
        assert_eq!(block.result.header.gas_used, U256::ZERO);
        assert_eq!(block.result.header.gas_limit, U256::from(u64::MAX));
        assert_eq!(block.result.header.difficulty, U256::ZERO);
        assert_eq!(block.result.header.size, None);
        assert_eq!(block.result.header.base_fee_per_gas, U256::from(1000000000));
        assert_eq!(block.result.header.mix_hash, H256::zero());
        assert_eq!(block.result.header.nonce, Some(H64::zero()));

        // Block
        assert_eq!(block.result.uncles, vec![]);
        // TODO: update tests when real data fetched
        assert_eq!(block.result.total_difficulty, U256::ZERO);
        assert_eq!(block.result.size, None);
        assert_eq!(block.result.base_fee_per_gas, None);

        let transactions = block.result.transactions;
        match transactions {
            BlockTransactions::Hashes(transactions) => {
                if let Some(first_tx) = transactions.first() {
                    let starknet_tx = FieldElement::from_str(
                        "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
                    )
                    .unwrap();
                    assert_eq!(first_tx, &H256::from_slice(&starknet_tx.to_bytes_be()));
                }
            }
            _ => {}
        }

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_get_block_by_number_hydrated_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByNumber\", \"params\": [\"latest\", true] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _block = res.json::<EthJsonRpcResponse<Block>>().await.unwrap();

        // TODO add test logic

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_get_block_by_number_not_hydrated_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByNumber\", \"params\": [\"latest\", false] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let block = res
            .json::<EthJsonRpcResponse<Block>>()
            .await
            .unwrap()
            .result;

        // Header data
        let starknet_block_hash = FieldElement::from_str(
            "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
        )
        .unwrap();
        assert_eq!(
            block.header.hash,
            Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
        );
        assert_eq!(block.header.number, Some(U256::from(19639)));

        let starknet_parent_hash = FieldElement::from_str(
            "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
        )
        .unwrap();
        let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
        assert_eq!(block.header.parent_hash, parent_hash);
        assert_eq!(block.header.uncles_hash, parent_hash);

        let starknet_sequencer = FieldElement::from_str(
            "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
        )
        .unwrap();
        let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
        assert_eq!(block.header.author, sequencer);
        assert_eq!(block.header.miner, sequencer);

        let starknet_new_root = FieldElement::from_str(
            "0x5549eb2dffae1d468fff16454cb2f44cdeea63ca79f56730304b170faecdd3b",
        )
        .unwrap();
        let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
        assert_eq!(block.header.state_root, state_root);

        assert_eq!(block.header.transactions_root, H256::zero());
        assert_eq!(block.header.receipts_root, H256::zero());

        assert_eq!(block.header.extra_data, Bytes::from(b"0x00"));
        assert_eq!(block.header.logs_bloom, Bloom::default());
        assert_eq!(block.header.timestamp, U256::from(1675496282));

        //TODO: update when real data fetched
        assert_eq!(block.header.gas_used, U256::ZERO);
        assert_eq!(block.header.gas_limit, U256::from(u64::MAX));
        assert_eq!(block.header.difficulty, U256::ZERO);
        assert_eq!(block.header.size, None);
        assert_eq!(block.header.base_fee_per_gas, U256::from(1000000000));
        assert_eq!(block.header.mix_hash, H256::zero());
        assert_eq!(block.header.nonce, Some(H64::zero()));

        // Block
        assert_eq!(block.uncles, vec![]);
        // TODO: update tests when real data fetched
        assert_eq!(block.total_difficulty, U256::ZERO);
        assert_eq!(block.size, None);
        assert_eq!(block.base_fee_per_gas, None);

        let transactions = block.transactions;
        match transactions {
            BlockTransactions::Hashes(transactions) => {
                if let Some(first_tx) = transactions.first() {
                    let starknet_tx = FieldElement::from_str(
                        "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
                    )
                    .unwrap();
                    assert_eq!(first_tx, &H256::from_slice(&starknet_tx.to_bytes_be()));
                }
            }
            _ => {}
        }
        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_hash_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockTransactionCountByHash\", \"params\": [\"0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e\"] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let transaction_count = res.json::<EthJsonRpcResponse<String>>().await.unwrap();
        assert_eq!(
            transaction_count.result,
            String::from(format!("0x{:0>64x}", 172))
        );
        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_number_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockTransactionCountByNumber\", \"params\": [\"latest\"] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let transaction_count = res.json::<EthJsonRpcResponse<String>>().await.unwrap();
        assert_eq!(
            transaction_count.result,
            String::from(format!("0x{:0>64x}", 172))
        );
        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_transaction_receipt_invoke_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getTransactionReceipt\", \"params\": [\"0x032e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258\"] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let transaction_receipt = res
            .json::<EthJsonRpcResponse<TransactionReceipt>>()
            .await
            .unwrap()
            .result;

        assert_eq!(
            transaction_receipt.transaction_hash,
            Some(H256::from_slice(
                &FieldElement::from_str(
                    "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );
        assert_eq!(
            transaction_receipt.block_hash,
            Some(H256::from_slice(
                &FieldElement::from_str(
                    "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );
        assert_eq!(transaction_receipt.block_number, Some(U256::from(19639)));
        assert_eq!(transaction_receipt.status_code, Some(U64::from(1)));

        assert_eq!(
            transaction_receipt.from,
            starknet_address_to_ethereum_address(
                &FieldElement::from_str(
                    "0x38240162a8eea5142d507ba750385497465a1bb55d4ca014bd34c8fdd5f63d8"
                )
                .unwrap()
            )
        );

        // TODO
        // assert_eq!(transaction_receipt.logs, None);
        // assert_eq!(transaction_receipt.contract_address, Some(U64::from(1)));

        // assert_eq!(transaction_receipt.transaction_index, None);
        // assert_eq!(transaction_receipt.to, None);
        // assert_eq!(transaction_receipt.cumulative_gas_used, U256::from(1000000));
        // assert_eq!(transaction_receipt.gas_used, None);
        // assert_eq!(transaction_receipt.logs_bloom, Bloom::default());
        // assert_eq!(transaction_receipt.state_root, None);
        // assert_eq!(transaction_receipt.effective_gas_price, U128::from(1000000));
        // assert_eq!(transaction_receipt.transaction_type, U256::from(0));

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_transaction_by_block_number_and_index_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
                .post("http://127.0.0.1:3030")
                .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getTransactionByBlockNumberAndIndex\", \"params\": [\"latest\", 1] }")
                .header("content-type", "application/json")
                .send()
                .await
                .unwrap();

        let transaction = res
            .json::<EthJsonRpcResponse<Transaction>>()
            .await
            .unwrap()
            .result;

        assert_eq!(
            transaction.hash,
            H256::from(
                &FieldElement::from_str(
                    "0x7c5df940744056d337c3de6e8f4500db4b9bfc821eb534b891555e90c39c048"
                )
                .unwrap()
                .to_bytes_be()
            )
        );
        assert_eq!(
            transaction.nonce,
            felt_to_u256(FieldElement::from_str("0x13").unwrap())
        );
        assert_eq!(
            transaction.block_hash,
            Some(H256::from(
                &FieldElement::from_str(
                    "0xa641151e9067e3919ca8d59191c473e2ecfb714578708c0cb0f99de000df05"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );
        assert_eq!(transaction.block_number, Some(U256::from(20129)));
        assert_eq!(
            transaction.from,
            starknet_address_to_ethereum_address(
                &FieldElement::from_str(
                    "0x13745d611a49179ab9b0fe943471f53ac9f0c8dc093db91c39ec5f67d20ab21"
                )
                .unwrap()
            )
        );

        let starknet_signature_r = FieldElement::from_str(
            "0x7d82e8c230ee321acefb67eaccfc55b7c90bf66c9af3b6975405f221587b974",
        )
        .unwrap();
        let starknet_signature_s = FieldElement::from_str(
            "0x5949c38b6a6f570ea1fdc840f93f875d46fe75619982ac300084ea0d27c4b14",
        )
        .unwrap();
        assert_eq!(
            transaction.r,
            felt_option_to_u256(Some(&starknet_signature_r)).unwrap()
        );
        assert_eq!(
            transaction.s,
            felt_option_to_u256(Some(&starknet_signature_s)).unwrap()
        );

        assert_eq!(transaction.creates, None);
        assert_eq!(transaction.public_key, None);
        assert_eq!(transaction.chain_id, Some(CHAIN_ID.into()));
        assert_eq!(transaction.access_list, None);
        assert_eq!(transaction.transaction_type, None);
        assert_eq!(transaction.standard_v, U256::from(0));

        // TODO
        // assert_eq!(transaction.input, None);

        // assert_eq!(transaction.to, None);
        // assert_eq!(transaction.transaction_index, None);
        // assert_eq!(transaction.value, U256::from(100));
        // assert_eq!(transaction.gas_price, None);
        // assert_eq!(transaction.max_fee_per_gas, None);
        // assert_eq!(transaction.max_priority_fee_per_gas, None);
        // assert_eq!(transaction.gas, U256::from(100));
        // assert_eq!(transaction.raw, None);

        server_handle.stop().unwrap();
    }

    #[tokio::test]
    async fn test_transaction_by_block_hash_and_index_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
                .post("http://127.0.0.1:3030")
                .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getTransactionByBlockHashAndIndex\", \"params\": [\"0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9\", 1] }")
                .header("content-type", "application/json")
                .send()
                .await
                .unwrap();

        let transaction = res
            .json::<EthJsonRpcResponse<Transaction>>()
            .await
            .unwrap()
            .result;

        assert_eq!(
            transaction.hash,
            H256::from(
                &FieldElement::from_str(
                    "0x7c5df940744056d337c3de6e8f4500db4b9bfc821eb534b891555e90c39c048"
                )
                .unwrap()
                .to_bytes_be()
            )
        );
        assert_eq!(
            transaction.nonce,
            felt_to_u256(FieldElement::from_str("0x13").unwrap())
        );
        assert_eq!(
            transaction.block_hash,
            Some(H256::from(
                &FieldElement::from_str(
                    "0xa641151e9067e3919ca8d59191c473e2ecfb714578708c0cb0f99de000df05"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );
        assert_eq!(transaction.block_number, Some(U256::from(20129)));
        assert_eq!(
            transaction.from,
            starknet_address_to_ethereum_address(
                &FieldElement::from_str(
                    "0x13745d611a49179ab9b0fe943471f53ac9f0c8dc093db91c39ec5f67d20ab21"
                )
                .unwrap()
            )
        );

        let starknet_signature_r = FieldElement::from_str(
            "0x7d82e8c230ee321acefb67eaccfc55b7c90bf66c9af3b6975405f221587b974",
        )
        .unwrap();
        let starknet_signature_s = FieldElement::from_str(
            "0x5949c38b6a6f570ea1fdc840f93f875d46fe75619982ac300084ea0d27c4b14",
        )
        .unwrap();
        assert_eq!(
            transaction.r,
            felt_option_to_u256(Some(&starknet_signature_r)).unwrap()
        );
        assert_eq!(
            transaction.s,
            felt_option_to_u256(Some(&starknet_signature_s)).unwrap()
        );

        assert_eq!(transaction.creates, None);
        assert_eq!(transaction.public_key, None);
        assert_eq!(transaction.chain_id, Some(CHAIN_ID.into()));
        assert_eq!(transaction.access_list, None);
        assert_eq!(transaction.transaction_type, None);
        assert_eq!(transaction.standard_v, U256::from(0));

        // TODO
        // assert_eq!(transaction.input, None);

        // assert_eq!(transaction.to, None);
        // assert_eq!(transaction.transaction_index, None);
        // assert_eq!(transaction.value, U256::from(100));
        // assert_eq!(transaction.gas_price, None);
        // assert_eq!(transaction.max_fee_per_gas, None);
        // assert_eq!(transaction.max_priority_fee_per_gas, None);
        // assert_eq!(transaction.gas, U256::from(100));
        // assert_eq!(transaction.raw, None);

        server_handle.stop().unwrap();
    }
}
