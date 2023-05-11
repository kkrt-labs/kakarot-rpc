#![recursion_limit = "256"]
mod assert_helpers;
mod utils;

#[cfg(test)]
mod tests {
    use crate::{
        assert_helpers::{assert_block, assert_block_header, assert_transaction},
        utils::setup_kakarot_eth_rpc,
    };
    use kakarot_rpc::eth_api::EthApiServer;
    use reth_primitives::{BlockNumberOrTag, H160, H256, U256, U64};
    use reth_rpc_types::Index;
    use serde_json::json;
    use starknet::{
        core::types::FieldElement, macros::felt,
        providers::jsonrpc::models::Transaction as StarknetTransaction,
    };
    use std::str::FromStr;

    fn get_test_tx() -> serde_json::Value {
        json!({
            "calldata":[
                "0x2",
                "0xf8",
                "0x72",
                "0x84",
                "0x4b",
                "0x4b",
                "0x52",
                "0x54",
                "0x82",
                "0xde",
                "0xad",
                "0x82",
                "0xde",
                "0xad",
                "0x82",
                "0xde",
                "0xad",
                "0x84",
                "0x3b",
                "0x9a",
                "0xca",
                "0x0",
                "0x94",
                "0x6f",
                "0x0",
                "0x9c",
                "0x55",
                "0x71",
                "0x35",
                "0xb9",
                "0x9e",
                "0x73",
                "0xbb",
                "0xd0",
                "0x1f",
                "0x45",
                "0x69",
                "0xd4",
                "0x15",
                "0xbc",
                "0x6a",
                "0x95",
                "0x10",
                "0x80",
                "0x84",
                "0x37",
                "0x13",
                "0x3",
                "0xc0",
                "0xc0",
                "0x80",
                "0xa0",
                "0x32",
                "0x50",
                "0x6f",
                "0x88",
                "0x9d",
                "0x51",
                "0x44",
                "0x7e",
                "0x4c",
                "0x52",
                "0x3f",
                "0x74",
                "0x49",
                "0x13",
                "0xa3",
                "0x58",
                "0xfc",
                "0xfe",
                "0x45",
                "0xa3",
                "0xdb",
                "0xf6",
                "0x24",
                "0xa9",
                "0xe6",
                "0x3b",
                "0xd9",
                "0xf3",
                "0x12",
                "0x80",
                "0x15",
                "0x28",
                "0xa0",
                "0x42",
                "0x62",
                "0x64",
                "0x2c",
                "0x44",
                "0x4f",
                "0xda",
                "0x47",
                "0xad",
                "0x53",
                "0xd6",
                "0x54",
                "0xc9",
                "0x78",
                "0x71",
                "0xb0",
                "0x81",
                "0x65",
                "0xb8",
                "0xf0",
                "0x88",
                "0xa5",
                "0xc8",
                "0x64",
                "0xc0",
                "0x75",
                "0x1f",
                "0x52",
                "0x76",
                "0x74",
                "0x5",
                "0x14"
             ],
             "max_fee":"0x28551b4c2e91c",
             "nonce":"0x04",
             "sender_address":"0x028d1467576420c7799e7fae5f5da963c0fce52e5723c854eee34c10f157a2df",
             "signature":[
                "0x4834178732bce2d497b4cecdfbd7710e010f72f07a66b8388baf4ee213bc17b",
                "0x6507bed64e0795c25d8d42eea2dadb5c7d41b6eddfb416256bd7de07bfd5892"
             ],
             "transaction_hash":"0x03ffcfea6eed902191033c88bded1e396a9aef4b88b32e6387eea30c83b84834",
             "type":"INVOKE",
             "version":"0x1"
        })
    }

    #[tokio::test]
    async fn test_block_number_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;

        let block_number = kakarot_rpc.block_number().await.unwrap();
        assert_eq!(block_number.as_u64(), 19640);
    }

    #[tokio::test]
    async fn test_get_block_by_hash_hydrated_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
                .unwrap();

        let hydrated = true;
        let block = kakarot_rpc
            .block_by_hash(hash, hydrated)
            .await
            .unwrap()
            .unwrap();

        let starknet_res = json!({
            "block_hash": "0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9",
            "block_number": 19612,
            "new_root": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "parent_hash": "0x137970a5417cf7d35eb4eeb04efe6312166f828eec76342338b0e3797ebf3c1",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1_675_461_581,
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
            &block,
            starknet_res.to_string(),
            starknet_txs.to_string(),
            true,
        );
        assert_block_header(&block, starknet_res.to_string(), true);
    }

    #[tokio::test]
    async fn test_get_block_by_hash_not_hydrated_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e")
                .unwrap();
        let hydrated = false;
        let block = kakarot_rpc
            .block_by_hash(hash, hydrated)
            .await
            .unwrap()
            .unwrap();

        let starknet_res = json!({
            "block_hash": "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
            "block_number": 19639,
            "new_root": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "parent_hash": "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1_675_496_282,
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
            &block,
            starknet_res.to_string(),
            starknet_txs.to_string(),
            false,
        );
        assert_block_header(&block, starknet_res.to_string(), false);
    }

    #[tokio::test]
    async fn test_get_block_by_number_hydrated_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;
        let hydrated = true;

        let block = kakarot_rpc
            .block_by_number(block_number, hydrated)
            .await
            .unwrap()
            .unwrap();

        let starknet_res = json!({
            "block_hash": "0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9",
            "block_number": 19612,
            "new_root": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "parent_hash": "0x137970a5417cf7d35eb4eeb04efe6312166f828eec76342338b0e3797ebf3c1",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1_675_461_581,
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
            &block,
            starknet_res.to_string(),
            starknet_txs.to_string(),
            true,
        );
        assert_block_header(&block, starknet_res.to_string(), true);
    }

    #[tokio::test]
    async fn test_get_block_by_number_not_hydrated_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;
        let hydrated = false;

        let block = kakarot_rpc
            .block_by_number(block_number, hydrated)
            .await
            .unwrap()
            .unwrap();

        let starknet_res = json!({
            "block_hash": "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e",
            "block_number": 19639,
            "new_root": "0x0000000000000000000000000000000000000000000000000000000000000000",
            "parent_hash": "0x13310ddd53ba41bd8b71dadbf1eb002c215ca8a790cb298d851ba7446e77d38",
            "sequencer_address": "0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9",
            "status": "ACCEPTED_ON_L2",
            "timestamp": 1_675_496_282,
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
            &block,
            starknet_res.to_string(),
            starknet_txs.to_string(),
            false,
        );
        assert_block_header(&block, starknet_res.to_string(), false);
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_hash_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
                .unwrap();

        let transaction_count = kakarot_rpc
            .block_transaction_count_by_hash(hash)
            .await
            .unwrap();
        assert_eq!(transaction_count.as_u64(), 42);
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_number_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;

        let transaction_count = kakarot_rpc
            .block_transaction_count_by_number(block_number)
            .await
            .unwrap();
        assert_eq!(transaction_count.as_u64(), 42);
    }

    #[tokio::test]
    async fn test_transaction_receipt_invoke_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x03ffcfea6eed902191033c88bded1e396a9aef4b88b32e6387eea30c83b84834")
                .unwrap();

        let transaction_receipt = kakarot_rpc
            .transaction_receipt(hash)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            transaction_receipt.transaction_hash,
            Some(H256::from_slice(
                &FieldElement::from_str(
                    "0x03ffcfea6eed902191033c88bded1e396a9aef4b88b32e6387eea30c83b84834"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );

        assert_eq!(
            transaction_receipt.block_hash,
            Some(H256::from_slice(
                &FieldElement::from_str(
                    "0x3a6aa138202f442b3d1f2d7702775a41ab78091578e2dfa4c93499ca380daa2"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );

        assert_eq!(
            U256::from(transaction_receipt.block_number.unwrap()),
            U256::from(803428)
        );
        assert_eq!(transaction_receipt.status_code, Some(U64::from(1)));

        assert_eq!(
            transaction_receipt.from,
            H160::from_str("0x54b288676b749def5fc10eb17244fe2c87375de1").unwrap()
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
    }

    #[tokio::test]
    async fn test_transaction_by_block_number_and_index_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;

        // workaround as Index does not implement new()
        let index: Index = Index::default();

        let transaction = kakarot_rpc
            .transaction_by_block_number_and_index(block_number, index)
            .await
            .unwrap()
            .unwrap();

        let starknet_tx = get_test_tx();
        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(H256::from(
                felt!("0x03a6aa138202f442b3d1f2d7702775a41ab78091578e2dfa4c93499ca380daa2")
                    .to_bytes_be()
            ))
        );

        assert_eq!(
            U256::from(transaction.block_number.unwrap()),
            U256::from(803428)
        );
    }

    #[tokio::test]
    async fn test_transaction_by_block_hash_and_index_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
                .unwrap();

        // workaround as Index does not implement new()
        let index: Index = Index::default();

        let transaction = kakarot_rpc
            .transaction_by_block_hash_and_index(hash, index)
            .await
            .unwrap()
            .unwrap();

        let starknet_tx = get_test_tx();

        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(H256::from(
                felt!("0x3a6aa138202f442b3d1f2d7702775a41ab78091578e2dfa4c93499ca380daa2")
                    .to_bytes_be()
            ))
        );
        assert_eq!(
            U256::from(transaction.block_number.unwrap()),
            U256::from(803428)
        );
    }
}
