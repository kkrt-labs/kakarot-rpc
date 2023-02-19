mod testing_helpers;

#[cfg(test)]
mod tests {
    use crate::testing_helpers::{assert_block, assert_block_header};
    use kakarot_rpc::test_utils::setup_rpc_server;
    use kakarot_rpc_core::{
        client::{
            constants::CHAIN_ID,
            types::{Block, Transaction},
        },
        helpers::{felt_option_to_u256, felt_to_u256, starknet_address_to_ethereum_address},
        utils::wiremock_utils::EthJsonRpcResponse,
    };
    use reth_primitives::{H256, U256, U64};
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

        let starknet_txs = json!({
            "transactions": [
                {
                    "calldata": [],
                    "max_fee": "0x1ec88b99c258ea",
                    "nonce": "0x34b",
                    "sender_address": "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
                    "signature": [
                        "0x5267c0d93467ddb5cfe0ab9db124ed5d57345e92a45111e7a08f8afa7666fae",
                        "0x622c1e743ae1060293085a9702ea1c6a7f642eb47b8eb9fb51ca0d156c5f5dd"
                    ],
                    "transaction_hash": "0x36b9fcadfafec68effe5c23bbacaf6197745a5e6317d3f174b80765942b5abb",
                    "type": "INVOKE",
                    "version": "0x1"
                }
            ]
        });

        assert_block(
            block.result.clone(),
            starknet_res.to_string(),
            starknet_txs.to_string(),
            true,
        );
        assert_block_header(block.result.clone(), starknet_res.to_string(), true);

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

        let starknet_res = json!({
            "block_hash": "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
            "block_number": 19639,
            "new_root": "0x5549eb2dffae1d468fff16454cb2f44cdeea63ca79f56730304b170faecdd3b",
            "parent_hash": "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1675496282,
        });

        let starknet_txs = json!({
            "transactions": [
                "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
                "0x1b7ec62724de1faba75fdc75cf11c1f855af33e4fe5f36d8a201237f3c9f257",
                "0x61e95439c1b3aaf19330e3d5feee59e2491b50972352aa18802bd87c5db4e6e",
                "0x68686063b3ada0375753c11f48a7d3c5874d8fabf9ec138f4cca5c14e81a14f",
                "0x9ac6108cdb3ef5faccbddaad1469e068d254efeacc8448382f1c0c41efb6c2",
                "0x17b9cfda6a162ef0d9f38d36ce61d3c24fa651e701f1aea30aa29d18be2fae8",
                "0x143eb205de403cc8dd8f2739a7f0aa61e0b4898d965031aaa493f450ab13650",
                "0x79fb1e4b6c481f305aeb26e5c97ca2262613d87eaffd959dc3f677537890749",
                "0x71b072c852797314c967830a21b7c41958c55e046c3d37e2ef4c5b93900afb9",
                "0x177a16b1369e92fccae5f8e55e98fe396acc4c7dbe93f39aea240d3e411a207",
                "0x217490d4b401e6b71306925882dd0611b029ca22438383147c4e98e632c2f3c",
            ]
        });

        assert_block(
            block.result.clone(),
            starknet_res.to_string(),
            starknet_txs.to_string(),
            false,
        );
        assert_block_header(block.result.clone(), starknet_res.to_string(), false);

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

        let starknet_txs = json!({
            "transactions": [
                {
                    "calldata": [],
                    "max_fee": "0x1ec88b99c258ea",
                    "nonce": "0x34b",
                    "sender_address": "0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
                    "signature": [
                        "0x5267c0d93467ddb5cfe0ab9db124ed5d57345e92a45111e7a08f8afa7666fae",
                        "0x622c1e743ae1060293085a9702ea1c6a7f642eb47b8eb9fb51ca0d156c5f5dd"
                    ],
                    "transaction_hash": "0x36b9fcadfafec68effe5c23bbacaf6197745a5e6317d3f174b80765942b5abb",
                    "type": "INVOKE",
                    "version": "0x1"
                }
            ]
        });

        assert_block(
            block.result.clone(),
            starknet_res.to_string(),
            starknet_txs.to_string(),
            true,
        );
        assert_block_header(block.result.clone(), starknet_res.to_string(), true);

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

        let starknet_res = json!({
            "block_hash": "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
            "block_number": 19639,
            "new_root": "0x5549eb2dffae1d468fff16454cb2f44cdeea63ca79f56730304b170faecdd3b",
            "parent_hash": "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1675496282,
        });

        let starknet_txs = json!({
            "transactions": [
                "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258",
                "0x1b7ec62724de1faba75fdc75cf11c1f855af33e4fe5f36d8a201237f3c9f257",
                "0x61e95439c1b3aaf19330e3d5feee59e2491b50972352aa18802bd87c5db4e6e",
                "0x68686063b3ada0375753c11f48a7d3c5874d8fabf9ec138f4cca5c14e81a14f",
                "0x9ac6108cdb3ef5faccbddaad1469e068d254efeacc8448382f1c0c41efb6c2",
                "0x17b9cfda6a162ef0d9f38d36ce61d3c24fa651e701f1aea30aa29d18be2fae8",
                "0x143eb205de403cc8dd8f2739a7f0aa61e0b4898d965031aaa493f450ab13650",
                "0x79fb1e4b6c481f305aeb26e5c97ca2262613d87eaffd959dc3f677537890749",
                "0x71b072c852797314c967830a21b7c41958c55e046c3d37e2ef4c5b93900afb9",
                "0x177a16b1369e92fccae5f8e55e98fe396acc4c7dbe93f39aea240d3e411a207",
                "0x217490d4b401e6b71306925882dd0611b029ca22438383147c4e98e632c2f3c",
            ]
        });

        assert_block(
            block.clone(),
            starknet_res.to_string(),
            starknet_txs.to_string(),
            false,
        );
        assert_block_header(block.clone(), starknet_res.to_string(), false);

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
