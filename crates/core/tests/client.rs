#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use kakarot_rpc_core::client::client_api::KakarotClient;
    use kakarot_rpc_core::mock::wiremock_utils::setup_mock_client_crate;
    use kakarot_rpc_core::models::convertible::{ConvertibleStarknetBlock, ConvertibleStarknetEvent};
    use kakarot_rpc_core::models::{BlockWithTxs, StarknetEvent};
    use reth_primitives::{Address, Bytes, H256};
    use reth_rpc_types::Log;
    use starknet::core::types::{BlockId, BlockTag, Event, FieldElement};
    use starknet::core::utils::get_selector_from_name;
    use starknet::providers::Provider;

    #[tokio::test]
    async fn test_starknet_block_to_eth_block() {
        let client = setup_mock_client_crate().await;
        let starknet_client = client.inner();
        let starknet_block = starknet_client.get_block_with_txs(BlockId::Tag(BlockTag::Latest)).await.unwrap();
        let eth_block = BlockWithTxs::new(starknet_block).to_eth_block(&client).await.unwrap();

        // TODO: Add more assertions & refactor into assert helpers
        // assert helpers should allow import of fixture file
        assert_eq!(
            eth_block.header.hash,
            Some(H256::from_slice(
                &FieldElement::from_hex_be("0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9")
                    .unwrap()
                    .to_bytes_be()
            ))
        )
    }

    #[tokio::test]
    async fn test_starknet_event_to_eth_log_success() {
        let client = setup_mock_client_crate().await;
        // given
        let data =
            vec![0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10];
        let felt_data: Vec<FieldElement> =
            data.iter().map(|&x| FieldElement::from_dec_str(&x.to_string()).unwrap()).collect();
        let bytes_data: Bytes = felt_data.iter().flat_map(|felt| felt.to_bytes_be()).collect::<Vec<u8>>().into();
        // see https://github.com/kkrt-labs/kakarot/blob/2133aaf58d5c8ae493c579570e43c9e011774309/tests/integration/solidity_contracts/PlainOpcodes/test_plain_opcodes.py#L120 this test generates the starknet event and ethereum log expected pair

        // FROM is hardcoded to the current hardcoded value of kakarot_contract
        let event3 = Event {
            from_address: FieldElement::from_hex_be(
                "0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75",
            )
            .unwrap(),
            keys: vec![
                FieldElement::from_dec_str("169107000779806480224941431033275202659").unwrap(),
                FieldElement::from_dec_str("119094765373898665007727700504125002894").unwrap(),
                FieldElement::from_dec_str("10").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("11").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("247666869351872231004050922759157890085502224190").unwrap(),
            ],
            data: felt_data,
        };

        let sn_event3 = StarknetEvent::new(event3);

        // when
        let resultant_eth_log3 = sn_event3.to_eth_log(&client).await.unwrap();

        // then
        let expected_eth_log3 = Log {
            address: Address::from_str("0x2B61c43A85bD35987C5311215e8288b823A6873E").unwrap(),
            topics: vec![
                H256::from_slice(
                    &hex::decode("5998d146b8109b9444e9bb13ae9a548e7f38d2db6e0da72afe22cefa3065bc63").unwrap(),
                ),
                H256::from_slice(
                    &hex::decode("000000000000000000000000000000000000000000000000000000000000000a").unwrap(),
                ),
                H256::from_slice(
                    &hex::decode("000000000000000000000000000000000000000000000000000000000000000b").unwrap(),
                ),
            ],
            data: bytes_data,
            transaction_hash: Option::None,
            block_hash: Option::None,
            block_number: Option::None,
            log_index: Option::None,
            transaction_index: Option::None,
            removed: false,
        };

        assert_eq!(expected_eth_log3, resultant_eth_log3);

        // see https://github.com/kkrt-labs/kakarot/blob/2133aaf58d5c8ae493c579570e43c9e011774309/tests/integration/solidity_contracts/PlainOpcodes/test_plain_opcodes.py#L124 this test generates the starknet event and ethereum log expected pair
        // given
        // FROM is hardcoded to the current hardcoded value of kakarot_contract
        let event4 = Event {
            from_address: FieldElement::from_hex_be(
                "0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75",
            )
            .unwrap(),
            keys: vec![
                FieldElement::from_dec_str("253936425291629012954210100230398563497").unwrap(),
                FieldElement::from_dec_str("171504579546982282416100792885946140532").unwrap(),
                FieldElement::from_dec_str("10").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("11").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("10").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("247666869351872231004050922759157890085502224190").unwrap(),
            ],
            data: vec![],
        };

        let sn_event4 = StarknetEvent::new(event4);

        // when
        let resultant_eth_log4 = sn_event4.to_eth_log(&client).await.unwrap();

        // then
        let expected_eth_log4 = Log {
            address: Address::from_str("0x2B61c43A85bD35987C5311215e8288b823A6873E").unwrap(),
            topics: vec![
                H256::from_slice(
                    &hex::decode("8106949def8a44172f54941ce774c774bf0a60652fafd614e9b6be2ca74a54a9").unwrap(),
                ),
                H256::from_slice(
                    &hex::decode("000000000000000000000000000000000000000000000000000000000000000a").unwrap(),
                ),
                H256::from_slice(
                    &hex::decode("000000000000000000000000000000000000000000000000000000000000000b").unwrap(),
                ),
                H256::from_slice(
                    &hex::decode("000000000000000000000000000000000000000000000000000000000000000a").unwrap(),
                ),
            ],
            data: Bytes::default(),
            transaction_hash: Option::None,
            block_hash: Option::None,
            block_number: Option::None,
            log_index: Option::None,
            transaction_index: Option::None,
            removed: false,
        };

        assert_eq!(expected_eth_log4, resultant_eth_log4);
    }

    #[tokio::test]
    async fn test_starknet_event_to_eth_log_failure_high_value_doesnt_exist() {
        let client = setup_mock_client_crate().await;

        let key_selector = get_selector_from_name("bbq_time").unwrap();
        // given
        let event = Event {
            from_address: FieldElement::from_hex_be("0xdeadbeef").unwrap(),
            // This keys vector only has one element for a pair, causing the high value to be missing.
            keys: vec![key_selector],
            data: vec![],
        };

        let sn_event = StarknetEvent::new(event);

        // when
        let resultant_eth_log = sn_event.to_eth_log(&client).await;

        // then
        // Expecting an error because the high value doesn't exist.
        match resultant_eth_log {
            Ok(_) => panic!("Expected an error due to missing high value, but got a result."),
            Err(err) => assert_eq!(err.to_string(), "Not an convertible event: High value doesn't exist"),
        }
    }

    #[tokio::test]
    async fn test_starknet_transaction_by_hash() {
        let client = setup_mock_client_crate().await;
        let starknet_tx = client
            .transaction_by_hash(
                H256::from_str("0x03204b4c0e379c3a5ccb80d08661d5a538e95e2960581c9faf7ebcf8ff5a7d3c").unwrap(),
            )
            .await;
        assert!(starknet_tx.is_ok());
    }
}
