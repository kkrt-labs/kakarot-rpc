mod assert_helpers;
mod utils;

#[cfg(test)]
mod tests {
    use crate::{
        assert_helpers::{assert_block, assert_block_header, assert_transaction},
        utils::setup_kakarot_eth_rpc,
    };
    use kakarot_rpc::eth_rpc::EthApiServer;
    use reth_primitives::{
        rpc::{BlockNumber, H256, U256, U64},
        H160, H256 as PrimitiveH256,
    };
    use reth_rpc_types::Index;
    use serde_json::json;
    use starknet::{
        core::types::FieldElement, macros::felt,
        providers::jsonrpc::models::Transaction as StarknetTransaction,
    };
    use std::str::FromStr;

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
        let block_number = BlockNumber::Latest;
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
        let block_number = BlockNumber::Latest;
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
            H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e")
                .unwrap();

        let transaction_count = kakarot_rpc
            .block_transaction_count_by_hash(hash)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(U256::from(transaction_count), U256::from(172));
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_number_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let block_number = BlockNumber::Latest;

        let transaction_count = kakarot_rpc
            .block_transaction_count_by_number(block_number)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(U256::from(transaction_count), U256::from(172));
    }

    #[tokio::test]
    async fn test_transaction_receipt_invoke_is_ok() {
        let kakarot_rpc = setup_kakarot_eth_rpc().await;
        let hash =
            H256::from_str("0x032e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258")
                .unwrap();

        let transaction_receipt = kakarot_rpc
            .transaction_receipt(hash)
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            transaction_receipt.transaction_hash,
            Some(PrimitiveH256::from_slice(
                &FieldElement::from_str(
                    "0x32e08cabc0f34678351953576e64f300add9034945c4bffd355de094fd97258"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );

        assert_eq!(
            transaction_receipt.block_hash,
            Some(PrimitiveH256::from_slice(
                &FieldElement::from_str(
                    "0x197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e"
                )
                .unwrap()
                .to_bytes_be()
            ))
        );

        assert_eq!(
            U256::from(transaction_receipt.block_number.unwrap()),
            U256::from(19639)
        );
        assert_eq!(transaction_receipt.status_code, Some(U64::from(1)));

        assert_eq!(
            transaction_receipt.from,
            H160::from_str("0x9296be4959e56b5df2200dbfa30594504a7fed61").unwrap()
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
        let block_number = BlockNumber::Latest;

        // workaround as Index does not implement new()
        let index: Index = Index::default();

        let transaction = kakarot_rpc
            .transaction_by_block_number_and_index(block_number, index)
            .await
            .unwrap()
            .unwrap();

        let starknet_tx = json!({
            "calldata":[
                "0x2",
                "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
                "0x219209e083275171774dab1df80982e9df2096516f06319c5c6d71ae0a8480c",
                "0x0",
                "0x3",
                "0x41fd22b238fa21cfcf5dd45a8548974d8263b3a531a60388411c5e230f97023",
                "0x3276861cf5e05d6daf8f352cabb47df623eb10c383ab742fcc7abea94d5c5cc",
                "0x3",
                "0x9",
                "0xc",
                "0x41fd22b238fa21cfcf5dd45a8548974d8263b3a531a60388411c5e230f97023",
                "0x47d3079",
                "0x0",
                "0x47d3079",
                "0x0",
                "0xaeda7450a74d13",
                "0x0",
                "0x2",
                "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
                "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                "0x13745d611a49179ab9b0fe943471f53ac9f0c8dc093db91c39ec5f67d20ab21",
                "0x63e78743"
             ],
             "max_fee":"0x28551b4c2e91c",
             "nonce":"0x13",
             "sender_address":"0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
             "signature":[
                "0x7d82e8c230ee321acefb67eaccfc55b7c90bf66c9af3b6975405f221587b974",
                "0x5949c38b6a6f570ea1fdc840f93f875d46fe75619982ac300084ea0d27c4b14"
             ],
             "transaction_hash":"0x7c5df940744056d337c3de6e8f4500db4b9bfc821eb534b891555e90c39c048",
             "type":"INVOKE",
             "version":"0x1"
        });

        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(PrimitiveH256::from(
                felt!("0xa641151e9067e3919ca8d59191c473e2ecfb714578708c0cb0f99de000df05")
                    .to_bytes_be()
            ))
        );

        assert_eq!(
            U256::from(transaction.block_number.unwrap()),
            U256::from(20129)
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

        let starknet_tx = json!({
            "calldata":[
                "0x2",
                "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
                "0x219209e083275171774dab1df80982e9df2096516f06319c5c6d71ae0a8480c",
                "0x0",
                "0x3",
                "0x41fd22b238fa21cfcf5dd45a8548974d8263b3a531a60388411c5e230f97023",
                "0x3276861cf5e05d6daf8f352cabb47df623eb10c383ab742fcc7abea94d5c5cc",
                "0x3",
                "0x9",
                "0xc",
                "0x41fd22b238fa21cfcf5dd45a8548974d8263b3a531a60388411c5e230f97023",
                "0x47d3079",
                "0x0",
                "0x47d3079",
                "0x0",
                "0xaeda7450a74d13",
                "0x0",
                "0x2",
                "0x53c91253bc9682c04929ca02ed00b3e423f6710d2ee7e0d5ebb06f3ecf368a8",
                "0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7",
                "0x13745d611a49179ab9b0fe943471f53ac9f0c8dc093db91c39ec5f67d20ab21",
                "0x63e78743"
             ],
             "max_fee":"0x28551b4c2e91c",
             "nonce":"0x13",
             "sender_address":"0xd90fd6aa27edd344c5cbe1fe999611416b268658e866a54265aaf50d9cf28d",
             "signature":[
                "0x7d82e8c230ee321acefb67eaccfc55b7c90bf66c9af3b6975405f221587b974",
                "0x5949c38b6a6f570ea1fdc840f93f875d46fe75619982ac300084ea0d27c4b14"
             ],
             "transaction_hash":"0x7c5df940744056d337c3de6e8f4500db4b9bfc821eb534b891555e90c39c048",
             "type":"INVOKE",
             "version":"0x1"
        });

        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(PrimitiveH256::from(
                felt!("0xa641151e9067e3919ca8d59191c473e2ecfb714578708c0cb0f99de000df05")
                    .to_bytes_be()
            ))
        );
        assert_eq!(
            U256::from(transaction.block_number.unwrap()),
            U256::from(20129)
        );
    }
}
