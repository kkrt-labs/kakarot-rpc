mod helpers;

#[cfg(test)]
mod tests {

    use std::str::FromStr;

    use dojo_test_utils::sequencer::TestSequencer;
    use kakarot_rpc_core::client::client_api::{KakarotEthApi, KakarotStarknetApi};
    use kakarot_rpc_core::client::config::StarknetConfig;
    use kakarot_rpc_core::client::errors::EthApiError;
    use kakarot_rpc_core::client::KakarotClient;
    use kakarot_rpc_core::mock::wiremock_utils::setup_mock_client_crate;
    use kakarot_rpc_core::models::block::BlockWithTxs;
    use kakarot_rpc_core::models::convertible::{ConvertibleStarknetBlock, ConvertibleStarknetEvent};
    use kakarot_rpc_core::models::event::StarknetEvent;
    use kakarot_rpc_core::models::felt::{Felt252Wrapper, Felt252WrapperError};
    use kakarot_rpc_core::models::ConversionError;
    use reth_primitives::hex_literal::hex;
    use reth_primitives::{Address, Bytes, H256};
    use reth_rpc_types::Log;
    use starknet::core::types::{BlockId, BlockTag, Event, FieldElement};
    use starknet::core::utils::get_selector_from_name;
    use starknet::providers::Provider;

    use crate::helpers::constants::{EVM_ADDRESS, EVM_PRIVATE_KEY};
    use crate::helpers::deploy_helpers::{create_raw_tx, init_kkrt_state};
    #[tokio::test]
    async fn use_test_sequencer_test() {
        let sn_test_sequencer = TestSequencer::start().await;

        let kkrt_eoa_private = H256::from_slice(&hex::decode(EVM_PRIVATE_KEY).unwrap());

        let expected_funded_amount = FieldElement::from_dec_str("100000000000000000").unwrap();

        let (kkrt_address, proxy_account_class_hash, _sn_eoa_address, deployed_addresses) =
            init_kkrt_state(&sn_test_sequencer, EVM_ADDRESS, kkrt_eoa_private, expected_funded_amount, "Counter.json")
                .await;

        let kakarot_client = KakarotClient::new(StarknetConfig::new(
            sn_test_sequencer.url().as_ref(),
            kkrt_address,
            proxy_account_class_hash,
        ))
        .unwrap();

        let deployed_balance = kakarot_client
            .balance(
                Address::from_slice(&hex::decode(EVM_ADDRESS.trim_start_matches("0x")).unwrap()),
                BlockId::Tag(BlockTag::Latest),
            )
            .await;

        // TODO: clean up how to compare this
        let deployed_balance = FieldElement::from_bytes_be(&deployed_balance.unwrap().to_be_bytes()).unwrap();

        assert_eq!(deployed_balance, expected_funded_amount);

        let Ok(counter_eth_addr) = ({
            let address : Felt252Wrapper = (*deployed_addresses.first().unwrap()).into();
            address.try_into()
        }) else {
            panic!("returned deploy address from kakarot failed to convert to address")
        };

        // asserts there is code at this address
        // let code_res = kkrt_eth_rpc.get_code(counter_eth_addr, Option::None).await;

        let inc_selector = hex!("371303c0");
        let inc_tx = create_raw_tx(inc_selector, kkrt_eoa_private, counter_eth_addr, vec![], 1u64);
        let inc_res = kakarot_client.send_transaction(inc_tx).await.unwrap();
        kakarot_client.transaction_receipt(inc_res).await.expect("increment receipt failed");

        let counter_selector = hex!("06661abd");
        let counter_val =
            kakarot_client.call_view(counter_eth_addr, counter_selector.into(), BlockId::Tag(BlockTag::Latest)).await;

        if let Ok(bytes) = counter_val {
            let num = *bytes.last().expect("Empty byte array");
            assert_eq!(num, 1);
        } else {
            panic!("Expected Ok, got {:?}", counter_val);
        }
    }

    #[tokio::test]
    async fn test_starknet_block_to_eth_block() {
        let client = setup_mock_client_crate().await;
        let starknet_client = client.starknet_provider();
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
        let kakarot_address =
            FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap();

        let event3 = Event {
            from_address: kakarot_address,
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
        let resultant_eth_log3 = sn_event3
            .to_eth_log(&client, Option::None, Option::None, Option::None, Option::None, Option::None)
            .unwrap();

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
        let event4 = Event {
            from_address: kakarot_address,
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
        let resultant_eth_log4 = sn_event4
            .to_eth_log(&client, Option::None, Option::None, Option::None, Option::None, Option::None)
            .unwrap();

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
    async fn test_starknet_event_to_eth_log_failure_from_address_not_kkrt_address() {
        let client = setup_mock_client_crate().await;

        let key_selector = get_selector_from_name("bbq_time").unwrap();
        // given
        let event = Event {
            // from address is not kkrt address
            from_address: FieldElement::from_hex_be("0xdeadbeef").unwrap(),
            keys: vec![key_selector],
            data: vec![],
        };

        let sn_event = StarknetEvent::new(event);

        // when
        let resultant_eth_log =
            sn_event.to_eth_log(&client, Option::None, Option::None, Option::None, Option::None, Option::None);

        // then
        // Expecting an error because the `from_address` of the starknet event is not the expected deployed
        // `kakarot_address'.
        match resultant_eth_log {
            Ok(_) => panic!("Expected an error due to wrong `from_address`, but got a result."),
            Err(EthApiError::OtherError(err)) => {
                assert_eq!(err.to_string(), "Kakarot Filter: Event is not part of Kakarot")
            }
            Err(_) => panic!("Expected a Kakarot Filter error, but got a different error."),
        }
    }

    #[tokio::test]
    async fn test_starknet_event_to_eth_log_failure_from_expected_evm_address_not_convertible() {
        let client = setup_mock_client_crate().await;

        let kakarot_address =
            FieldElement::from_hex_be("0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75").unwrap();

        // the felt that is supposed to represent an ethereum address is larger than a H160
        let large_field_element =
            FieldElement::from_dec_str("1606938044258990275541962092341162602522202993782792835301376").unwrap();

        // given
        let event = Event {
            from_address: kakarot_address,
            keys: vec![
                FieldElement::from_dec_str("253936425291629012954210100230398563497").unwrap(),
                FieldElement::from_dec_str("171504579546982282416100792885946140532").unwrap(),
                FieldElement::from_dec_str("10").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("11").unwrap(),
                FieldElement::ZERO,
                FieldElement::from_dec_str("10").unwrap(),
                FieldElement::ZERO,
                large_field_element, // Use the large FieldElement here.
            ],
            data: vec![],
        };

        let sn_event = StarknetEvent::new(event);

        // when
        let resultant_eth_log =
            sn_event.to_eth_log(&client, Option::None, Option::None, Option::None, Option::None, Option::None);

        // then
        match resultant_eth_log {
            Ok(_) => panic!("Expected an error due to wrong `from_address`, but got a result."),
            Err(EthApiError::ConversionError(ConversionError::ToEthereumAddressError)) => {
                // Test passes if we match this far.
            }
            Err(_) => panic!("Expected a ToEthereumAddressError, but got a different error."),
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
