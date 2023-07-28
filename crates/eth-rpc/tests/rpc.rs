#![recursion_limit = "1024"]
#[macro_use]
mod utils;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use kakarot_rpc::api::eth_api::EthApiServer;
    use kakarot_rpc_core::mock::assert_helpers::{assert_block, assert_block_header, assert_transaction};
    use reth_primitives::{BlockNumberOrTag, H160, H256, U256, U64};
    use reth_rpc_types::Index;
    use serde_json::json;
    use starknet::core::types::{FieldElement, Transaction as StarknetTransaction};
    use starknet::macros::felt;

    use crate::utils::setup_mock_eth_rpc;

    fn get_test_tx() -> serde_json::Value {
        json!({
        "calldata": [
          "0x01",
          "0x06eac8dd0d230c4b37f46bf4c20fb2dc21cd55f87791e2a76beae8059bd8e5e6",
          "0x03f74ebc1d04a8af0c3aab297dae7a62925043ee729e7c2d649161e12e2cfbdb",
          "0x00",
          "0x02be",
          "0x02be",
          "0x02",
          "0x0f9",
          "0x02",
          "0x0ba",
          "0x084",
          "0x04b",
          "0x04b",
          "0x052",
          "0x054",
          "0x082",
          "0x0de",
          "0x0ad",
          "0x082",
          "0x0de",
          "0x0ad",
          "0x082",
          "0x0de",
          "0x0ad",
          "0x082",
          "0x0de",
          "0x0ad",
          "0x080",
          "0x080",
          "0x0b9",
          "0x02",
          "0x060",
          "0x060",
          "0x080",
          "0x060",
          "0x040",
          "0x052",
          "0x034",
          "0x080",
          "0x015",
          "0x061",
          "0x00",
          "0x010",
          "0x057",
          "0x060",
          "0x00",
          "0x080",
          "0x0fd",
          "0x05b",
          "0x050",
          "0x060",
          "0x00",
          "0x080",
          "0x055",
          "0x061",
          "0x02",
          "0x03c",
          "0x080",
          "0x061",
          "0x00",
          "0x024",
          "0x060",
          "0x00",
          "0x039",
          "0x060",
          "0x00",
          "0x0f3",
          "0x0fe",
          "0x060",
          "0x080",
          "0x060",
          "0x040",
          "0x052",
          "0x034",
          "0x080",
          "0x015",
          "0x061",
          "0x00",
          "0x010",
          "0x057",
          "0x060",
          "0x00",
          "0x080",
          "0x0fd",
          "0x05b",
          "0x050",
          "0x060",
          "0x04",
          "0x036",
          "0x010",
          "0x061",
          "0x00",
          "0x062",
          "0x057",
          "0x060",
          "0x00",
          "0x035",
          "0x060",
          "0x0e0",
          "0x01c",
          "0x080",
          "0x063",
          "0x06",
          "0x066",
          "0x01a",
          "0x0bd",
          "0x014",
          "0x061",
          "0x00",
          "0x067",
          "0x057",
          "0x080",
          "0x063",
          "0x037",
          "0x013",
          "0x03",
          "0x0c0",
          "0x014",
          "0x061",
          "0x00",
          "0x082",
          "0x057",
          "0x080",
          "0x063",
          "0x07c",
          "0x050",
          "0x07c",
          "0x0bd",
          "0x014",
          "0x061",
          "0x00",
          "0x08c",
          "0x057",
          "0x080",
          "0x063",
          "0x0b3",
          "0x0bc",
          "0x0fa",
          "0x082",
          "0x014",
          "0x061",
          "0x00",
          "0x094",
          "0x057",
          "0x080",
          "0x063",
          "0x0d8",
          "0x026",
          "0x0f8",
          "0x08f",
          "0x014",
          "0x061",
          "0x00",
          "0x09c",
          "0x057",
          "0x080",
          "0x063",
          "0x0f0",
          "0x070",
          "0x07e",
          "0x0a9",
          "0x014",
          "0x061",
          "0x00",
          "0x0a5",
          "0x057",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x0fd",
          "0x05b",
          "0x061",
          "0x00",
          "0x070",
          "0x060",
          "0x00",
          "0x054",
          "0x081",
          "0x056",
          "0x05b",
          "0x060",
          "0x040",
          "0x051",
          "0x090",
          "0x081",
          "0x052",
          "0x060",
          "0x020",
          "0x01",
          "0x060",
          "0x040",
          "0x051",
          "0x080",
          "0x091",
          "0x03",
          "0x090",
          "0x0f3",
          "0x05b",
          "0x061",
          "0x00",
          "0x08a",
          "0x061",
          "0x00",
          "0x0ad",
          "0x056",
          "0x05b",
          "0x00",
          "0x05b",
          "0x061",
          "0x00",
          "0x08a",
          "0x061",
          "0x00",
          "0x0c6",
          "0x056",
          "0x05b",
          "0x061",
          "0x00",
          "0x08a",
          "0x061",
          "0x01",
          "0x06",
          "0x056",
          "0x05b",
          "0x061",
          "0x00",
          "0x08a",
          "0x060",
          "0x00",
          "0x080",
          "0x055",
          "0x056",
          "0x05b",
          "0x061",
          "0x00",
          "0x08a",
          "0x061",
          "0x01",
          "0x039",
          "0x056",
          "0x05b",
          "0x060",
          "0x01",
          "0x060",
          "0x00",
          "0x080",
          "0x082",
          "0x082",
          "0x054",
          "0x061",
          "0x00",
          "0x0bf",
          "0x091",
          "0x090",
          "0x061",
          "0x01",
          "0x07c",
          "0x056",
          "0x05b",
          "0x090",
          "0x091",
          "0x055",
          "0x050",
          "0x050",
          "0x056",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x054",
          "0x011",
          "0x061",
          "0x00",
          "0x0f0",
          "0x057",
          "0x060",
          "0x040",
          "0x051",
          "0x062",
          "0x046",
          "0x01b",
          "0x0cd",
          "0x060",
          "0x0e5",
          "0x01b",
          "0x081",
          "0x052",
          "0x060",
          "0x04",
          "0x01",
          "0x061",
          "0x00",
          "0x0e7",
          "0x090",
          "0x061",
          "0x01",
          "0x095",
          "0x056",
          "0x05b",
          "0x060",
          "0x040",
          "0x051",
          "0x080",
          "0x091",
          "0x03",
          "0x090",
          "0x0fd",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x054",
          "0x090",
          "0x080",
          "0x061",
          "0x00",
          "0x0ff",
          "0x083",
          "0x061",
          "0x01",
          "0x0dc",
          "0x056",
          "0x05b",
          "0x091",
          "0x090",
          "0x050",
          "0x055",
          "0x050",
          "0x056",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x054",
          "0x011",
          "0x061",
          "0x01",
          "0x027",
          "0x057",
          "0x060",
          "0x040",
          "0x051",
          "0x062",
          "0x046",
          "0x01b",
          "0x0cd",
          "0x060",
          "0x0e5",
          "0x01b",
          "0x081",
          "0x052",
          "0x060",
          "0x04",
          "0x01",
          "0x061",
          "0x00",
          "0x0e7",
          "0x090",
          "0x061",
          "0x01",
          "0x095",
          "0x056",
          "0x05b",
          "0x060",
          "0x01",
          "0x060",
          "0x00",
          "0x080",
          "0x082",
          "0x082",
          "0x054",
          "0x061",
          "0x00",
          "0x0bf",
          "0x091",
          "0x090",
          "0x061",
          "0x01",
          "0x0f3",
          "0x056",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x054",
          "0x011",
          "0x061",
          "0x01",
          "0x05a",
          "0x057",
          "0x060",
          "0x040",
          "0x051",
          "0x062",
          "0x046",
          "0x01b",
          "0x0cd",
          "0x060",
          "0x0e5",
          "0x01b",
          "0x081",
          "0x052",
          "0x060",
          "0x04",
          "0x01",
          "0x061",
          "0x00",
          "0x0e7",
          "0x090",
          "0x061",
          "0x01",
          "0x095",
          "0x056",
          "0x05b",
          "0x060",
          "0x00",
          "0x080",
          "0x054",
          "0x060",
          "0x00",
          "0x019",
          "0x01",
          "0x090",
          "0x055",
          "0x056",
          "0x05b",
          "0x063",
          "0x04e",
          "0x048",
          "0x07b",
          "0x071",
          "0x060",
          "0x0e0",
          "0x01b",
          "0x060",
          "0x00",
          "0x052",
          "0x060",
          "0x011",
          "0x060",
          "0x04",
          "0x052",
          "0x060",
          "0x024",
          "0x060",
          "0x00",
          "0x0fd",
          "0x05b",
          "0x080",
          "0x082",
          "0x01",
          "0x080",
          "0x082",
          "0x011",
          "0x015",
          "0x061",
          "0x01",
          "0x08f",
          "0x057",
          "0x061",
          "0x01",
          "0x08f",
          "0x061",
          "0x01",
          "0x066",
          "0x056",
          "0x05b",
          "0x092",
          "0x091",
          "0x050",
          "0x050",
          "0x056",
          "0x05b",
          "0x060",
          "0x020",
          "0x080",
          "0x082",
          "0x052",
          "0x060",
          "0x027",
          "0x090",
          "0x082",
          "0x01",
          "0x052",
          "0x07f",
          "0x063",
          "0x06f",
          "0x075",
          "0x06e",
          "0x074",
          "0x020",
          "0x073",
          "0x068",
          "0x06f",
          "0x075",
          "0x06c",
          "0x064",
          "0x020",
          "0x062",
          "0x065",
          "0x020",
          "0x073",
          "0x074",
          "0x072",
          "0x069",
          "0x063",
          "0x074",
          "0x06c",
          "0x079",
          "0x020",
          "0x067",
          "0x072",
          "0x065",
          "0x061",
          "0x074",
          "0x065",
          "0x072",
          "0x060",
          "0x040",
          "0x082",
          "0x01",
          "0x052",
          "0x066",
          "0x02",
          "0x07",
          "0x046",
          "0x086",
          "0x016",
          "0x0e2",
          "0x03",
          "0x060",
          "0x0cc",
          "0x01b",
          "0x060",
          "0x060",
          "0x082",
          "0x01",
          "0x052",
          "0x060",
          "0x080",
          "0x01",
          "0x090",
          "0x056",
          "0x05b",
          "0x060",
          "0x00",
          "0x081",
          "0x061",
          "0x01",
          "0x0eb",
          "0x057",
          "0x061",
          "0x01",
          "0x0eb",
          "0x061",
          "0x01",
          "0x066",
          "0x056",
          "0x05b",
          "0x050",
          "0x060",
          "0x00",
          "0x019",
          "0x01",
          "0x090",
          "0x056",
          "0x05b",
          "0x081",
          "0x081",
          "0x03",
          "0x081",
          "0x081",
          "0x011",
          "0x015",
          "0x061",
          "0x01",
          "0x08f",
          "0x057",
          "0x061",
          "0x01",
          "0x08f",
          "0x061",
          "0x01",
          "0x066",
          "0x056",
          "0x0fe",
          "0x0a2",
          "0x064",
          "0x069",
          "0x070",
          "0x066",
          "0x073",
          "0x058",
          "0x022",
          "0x012",
          "0x020",
          "0x030",
          "0x091",
          "0x0d3",
          "0x04e",
          "0x06c",
          "0x0be",
          "0x0bc",
          "0x053",
          "0x019",
          "0x08d",
          "0x04c",
          "0x0d",
          "0x09",
          "0x078",
          "0x06b",
          "0x051",
          "0x042",
          "0x03a",
          "0x07a",
          "0x0e0",
          "0x0de",
          "0x031",
          "0x044",
          "0x056",
          "0x0c7",
          "0x04c",
          "0x068",
          "0x0aa",
          "0x0cc",
          "0x0c3",
          "0x011",
          "0x0e3",
          "0x064",
          "0x073",
          "0x06f",
          "0x06c",
          "0x063",
          "0x043",
          "0x00",
          "0x08",
          "0x011",
          "0x00",
          "0x033",
          "0x0c0",
          "0x01",
          "0x0a0",
          "0x05e",
          "0x06a",
          "0x035",
          "0x0e5",
          "0x037",
          "0x0e8",
          "0x0d9",
          "0x09c",
          "0x081",
          "0x0bf",
          "0x02d",
          "0x04e",
          "0x07e",
          "0x08a",
          "0x041",
          "0x0e",
          "0x07f",
          "0x06f",
          "0x03f",
          "0x08b",
          "0x01f",
          "0x07",
          "0x0ed",
          "0x0c2",
          "0x08b",
          "0x0f2",
          "0x026",
          "0x0d3",
          "0x0ac",
          "0x02c",
          "0x0ae",
          "0x012",
          "0x0a0",
          "0x019",
          "0x010",
          "0x0d7",
          "0x0b4",
          "0x078",
          "0x04e",
          "0x073",
          "0x047",
          "0x0a6",
          "0x0c7",
          "0x0dc",
          "0x0cf",
          "0x08b",
          "0x080",
          "0x051",
          "0x0c0",
          "0x06f",
          "0x09",
          "0x013",
          "0x047",
          "0x0eb",
          "0x04a",
          "0x04a",
          "0x02f",
          "0x060",
          "0x092",
          "0x0f1",
          "0x054",
          "0x01c",
          "0x0b6",
          "0x02d",
          "0x0e7"
        ],
        "max_fee": "0x016345785d8a0000",
        "nonce": "0x00",
        "sender_address": "0x0744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485",
        "signature": [
          "0x076e91a117d68549b7c7be395f1bd01596372f2ac631bd6ce6202430654434e",
          "0x04ef32bc4fd31910b365bff935637cc2b4a084c73a9bbd91e6f5e4fd6062deb0"
        ],
        "transaction_hash": "0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c",
        "type": "INVOKE",
        "version": "0x1"
        })
    }

    #[tokio::test]
    async fn test_block_number_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;

        let block_number = kakarot_rpc.block_number().await.unwrap();
        assert_eq!(block_number.as_u64(), 19640);
    }

    #[tokio::test]
    async fn test_get_block_by_hash_hydrated_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let hash = H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9").unwrap();

        let hydrated = true;
        let block = kakarot_rpc.block_by_hash(hash, hydrated).await.unwrap().unwrap();

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
                    "max_fee": "0x016345785d8a0000",
                    "nonce": "0x00",
                    "sender_address": "0x0744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485",
                    "signature": [
                      "0x076e91a117d68549b7c7be395f1bd01596372f2ac631bd6ce6202430654434e",
                      "0x04ef32bc4fd31910b365bff935637cc2b4a084c73a9bbd91e6f5e4fd6062deb0"
                    ],
                    "transaction_hash": "0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c",
                    "type": "INVOKE",
                    "version": "0x1"
                }
            ]
        });

        assert_block(&block, starknet_res.to_string(), starknet_txs.to_string(), true);
        assert_block_header(&block, starknet_res.to_string(), true);
    }

    #[tokio::test]
    async fn test_get_block_by_hash_not_hydrated_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let hash = H256::from_str("0x0197be2810df6b5eedd5d9e468b200d0b845b642b81a44755e19047f08cc8c6e").unwrap();
        let hydrated = false;
        let block = kakarot_rpc.block_by_hash(hash, hydrated).await.unwrap().unwrap();

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

        assert_block(&block, starknet_res.to_string(), starknet_txs.to_string(), false);
        assert_block_header(&block, starknet_res.to_string(), false);
    }

    #[tokio::test]
    async fn test_get_block_by_number_hydrated_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;
        let hydrated = true;

        let block = kakarot_rpc.block_by_number(block_number, hydrated).await.unwrap().unwrap();

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
                    "max_fee": "0x016345785d8a0000",
                    "nonce": "0x00",
                    "sender_address": "0x0744ed080b42c8883a7e31cd11a14b7ae9ef27698b785486bb75cd116c8f1485",
                    "signature": [
                      "0x076e91a117d68549b7c7be395f1bd01596372f2ac631bd6ce6202430654434e",
                      "0x04ef32bc4fd31910b365bff935637cc2b4a084c73a9bbd91e6f5e4fd6062deb0"
                    ],
                    "transaction_hash": "0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c",
                    "type": "INVOKE",
                    "version": "0x1"
                }
            ]
        });

        assert_block(&block, starknet_res.to_string(), starknet_txs.to_string(), true);
        assert_block_header(&block, starknet_res.to_string(), true);
    }

    #[tokio::test]
    async fn test_get_block_by_number_not_hydrated_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;
        let hydrated = false;

        let block = kakarot_rpc.block_by_number(block_number, hydrated).await.unwrap().unwrap();

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

        assert_block(&block, starknet_res.to_string(), starknet_txs.to_string(), false);
        assert_block_header(&block, starknet_res.to_string(), false);
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_hash_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let hash = H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9").unwrap();

        let transaction_count = kakarot_rpc.block_transaction_count_by_hash(hash).await.unwrap();
        assert_eq!(transaction_count.as_u64(), 16);
    }

    #[tokio::test]
    async fn test_block_transaction_count_by_number_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;

        let transaction_count = kakarot_rpc.block_transaction_count_by_number(block_number).await.unwrap();
        assert_eq!(transaction_count.as_u64(), 16);
    }

    #[tokio::test]
    async fn test_transaction_receipt_invoke_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let hash = H256::from_str("0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c").unwrap();
        let transaction_receipt = kakarot_rpc.transaction_receipt(hash).await.unwrap().unwrap();

        assert_eq!(
            transaction_receipt.transaction_hash,
            Some(H256::from_slice(
                &FieldElement::from_str("0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c")
                    .unwrap()
                    .to_bytes_be()
            ))
        );

        assert_eq!(
            transaction_receipt.block_hash,
            Some(H256::from_slice(
                &FieldElement::from_str("0x00000000000000000000000000000000000000000000000000000000000000d")
                    .unwrap()
                    .to_bytes_be()
            ))
        );

        assert_eq!(U256::from(transaction_receipt.block_number.unwrap()), U256::from(13));
        assert_eq!(transaction_receipt.status_code, Some(U64::from(1)));

        assert_eq!(transaction_receipt.from, H160::from_str("0x54b288676b749def5fc10eb17244fe2c87375de1").unwrap());

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
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let block_number = BlockNumberOrTag::Latest;

        // workaround as Index does not implement new()
        let index: Index = Index::default();

        let transaction =
            kakarot_rpc.transaction_by_block_number_and_index(block_number, index).await.unwrap().unwrap();

        let starknet_tx = get_test_tx();
        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(H256::from(felt!("0x000000000000000000000000000000000000000000000000000000000000000d").to_bytes_be()))
        );

        assert_eq!(U256::from(transaction.block_number.unwrap()), U256::from(13));
    }

    #[tokio::test]
    async fn test_transaction_by_block_hash_and_index_is_ok() {
        let kakarot_rpc = setup_mock_eth_rpc().await;
        let hash = H256::from_str("0x0449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9").unwrap();

        // workaround as Index does not implement new()
        let index: Index = Index::default();

        let transaction = kakarot_rpc.transaction_by_block_hash_and_index(hash, index).await.unwrap().unwrap();

        let starknet_tx = get_test_tx();

        assert_transaction(
            transaction.clone(),
            serde_json::from_str::<StarknetTransaction>(&starknet_tx.to_string()).unwrap(),
        );

        assert_eq!(
            transaction.block_hash,
            Some(H256::from(felt!("0x000000000000000000000000000000000000000000000000000000000000000d").to_bytes_be()))
        );
        assert_eq!(U256::from(transaction.block_number.unwrap()), U256::from(13));
    }
}
