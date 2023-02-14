#[cfg(test)]
mod tests {
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
        // Header data
        let starknet_block_hash = FieldElement::from_str(
            "0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9",
        )
        .unwrap();
        assert_eq!(
            block.result.header.hash,
            Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
        );
        assert_eq!(block.result.header.number, Some(U256::from(19612)));

        let starknet_parent_hash = FieldElement::from_str(
            "0x137970a5417cf7d35eb4eeb04efe6312166f828eec76342338b0e3797ebf3c1",
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
            "0x67cde84ecff30c4ca55cb46df37940df87a94cc416cb893eaa9fb4fb67ec513",
        )
        .unwrap();
        let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
        assert_eq!(block.result.header.state_root, state_root);

        assert_eq!(
            block.result.header.transactions_root,
            H256::from_slice(
                &"0xac91334ba861cb94cba2b1fd63df7e87c15ca73666201abd10b5462255a5c642".as_bytes()
                    [1..33],
            )
        );
        assert_eq!(
            block.result.header.receipts_root,
            H256::from_slice(
                &"0xf2c8755adf35e78ffa84999e48aba628e775bb7be3c70209738d736b67a9b549".as_bytes()
                    [1..33],
            )
        );

        assert_eq!(block.result.header.extra_data, Bytes::from(b"0x00"));
        assert_eq!(block.result.header.logs_bloom, Bloom::default());
        assert_eq!(block.result.header.timestamp, U256::from(1675461581));

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
            BlockTransactions::Full(transactions) => {
                // Test InvokeV1 Transaction result
                if let Some(first_tx) = transactions.first() {
                    assert_eq!(first_tx.block_number, Some(U256::from(19612)));
                    assert_eq!(
                        first_tx.block_hash,
                        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
                    );

                    let starknet_hash = FieldElement::from_str(
                        "0x36b9fcadfafec68effe5c23bbacaf6197745a5e6317d3f174b80765942b5abb",
                    )
                    .unwrap();
                    assert_eq!(
                        first_tx.hash,
                        H256::from_slice(&starknet_hash.to_bytes_be())
                    );

                    let starknet_nonce = FieldElement::from_hex_be(&"0x34b".to_string()).unwrap();
                    assert_eq!(first_tx.nonce, felt_to_u256(starknet_nonce));

                    assert_eq!(
                        first_tx.from,
                        starknet_address_to_ethereum_address(
                            &FieldElement::from_str(
                                "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d"
                            )
                            .unwrap()
                        )
                    );

                    assert_eq!(first_tx.chain_id, Some(CHAIN_ID.into()));
                    assert_eq!(first_tx.standard_v, U256::from(0));
                    assert_eq!(first_tx.creates, None);
                    assert_eq!(first_tx.access_list, None);
                    assert_eq!(first_tx.transaction_type, None);

                    let starknet_signature_r = FieldElement::from_str(
                        "0x5267c0d93467ddb5cfe0ab9db124ed5d57345e92a45111e7a08f8afa7666fae",
                    )
                    .unwrap();
                    let starknet_signature_s = FieldElement::from_str(
                        "0x622c1e743ae1060293085a9702ea1c6a7f642eb47b8eb9fb51ca0d156c5f5dd",
                    )
                    .unwrap();
                    assert_eq!(
                        first_tx.r,
                        felt_option_to_u256(Some(&starknet_signature_r)).unwrap()
                    );
                    assert_eq!(
                        first_tx.s,
                        felt_option_to_u256(Some(&starknet_signature_s)).unwrap()
                    );

                    // TODO update when real data fetched
                    assert_eq!(first_tx.to, None);
                    assert_eq!(first_tx.value, U256::from(100));
                    assert_eq!(first_tx.gas, U256::from(100));
                    assert_eq!(first_tx.gas_price, None);

                    // TODO test first_tx.input
                }

                // TODO test InvokeV0, deploy and deployAccount transaction results
            }
            _ => {}
        }
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
