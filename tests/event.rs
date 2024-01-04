mod test_utils;
use kakarot_rpc::models::event::StarknetEvent;
use reth_primitives::{H256, U256};
use reth_rpc_types::Log;
use rstest::*;
use starknet::core::types::Event;
use test_utils::fixtures::katana;
use test_utils::sequencer::Katana;

use crate::test_utils::constants::KAKAROT_ADDRESS;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_log_log3(#[future] katana: Katana) {
    // Given
    let mut event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log3.json")).unwrap();
    event.from_address = *KAKAROT_ADDRESS;
    let starknet_event = StarknetEvent::new(event);

    let client = katana.client();

    // When
    let eth_log = starknet_event.to_eth_log(client, None, None, None, None, None).unwrap();

    // Then
    let expected: Log = serde_json::from_str(include_str!("test_data/conversion/eth/event_log3.json")).unwrap();
    assert_eq!(expected, eth_log);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_log_log4(#[future] katana: Katana) {
    // Given
    let mut event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log4.json")).unwrap();
    event.from_address = *KAKAROT_ADDRESS;
    let starknet_event = StarknetEvent::new(event);

    let client = katana.client();

    // When
    let eth_log = starknet_event.to_eth_log(client, None, None, None, None, None).unwrap();

    // Then
    let expected: Log = serde_json::from_str(include_str!("test_data/conversion/eth/event_log4.json")).unwrap();
    assert_eq!(expected, eth_log);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "KakarotDataFilteringError(\"Event\")")]
async fn test_to_eth_log_should_fail_on_from_address_not_kakarot_address(#[future] katana: Katana) {
    // Given
    let mut event: Event =
        serde_json::from_str(include_str!("test_data/conversion/starknet/event_invalid_from_address.json")).unwrap();
    event.from_address = *KAKAROT_ADDRESS;

    let starknet_event = StarknetEvent::new(event);

    let client = katana.client();

    // When
    starknet_event.to_eth_log(client, None, None, None, None, None).unwrap();
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
#[should_panic(expected = "ConversionError(\"failed to convert Felt252Wrapper to Ethereum address: the value \
                               exceeds the maximum size of an Ethereum address\")")]
async fn test_to_eth_log_should_fail_on_key_not_convertible_to_eth_address(#[future] katana: Katana) {
    // Given
    let mut event: Event =
        serde_json::from_str(include_str!("test_data/conversion/starknet/event_invalid_key.json")).unwrap();
    event.from_address = *KAKAROT_ADDRESS;

    let starknet_event = StarknetEvent::new(event);

    let client = katana.client();

    // When
    starknet_event.to_eth_log(client, None, None, None, None, None).unwrap();
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_log_with_optional_parameters(#[future] katana: Katana) {
    // Given
    let mut event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log3.json")).unwrap();
    event.from_address = *KAKAROT_ADDRESS;
    let starknet_event = StarknetEvent::new(event);

    let client = katana.client();

    // When
    let block_hash = Some(H256::from_low_u64_be(0xdeadbeef));
    let block_number = Some(U256::from(0x1));
    let transaction_hash = Some(H256::from_low_u64_be(0x12));
    let transaction_index = Some(U256::from(0x123));
    let log_index = Some(U256::from(0x1234));
    let eth_event = starknet_event
        .to_eth_log(client, block_hash, block_number, transaction_hash, log_index, transaction_index)
        .unwrap();

    // Then
    let expected: Log =
        serde_json::from_str(include_str!("test_data/conversion/eth/event_log3_with_optionals.json")).unwrap();
    assert_eq!(expected, eth_event);
}
