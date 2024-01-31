#![cfg(feature = "testing")]
use kakarot_rpc::{
    eth_provider::provider::EthereumProvider as _,
    models::felt::Felt252Wrapper,
    test_utils::{eoa::Eoa as _, fixtures::setup, sequencer::Katana},
    test_utils::{
        evm_contract::KakarotEvmContract,
        fixtures::{counter, katana},
    },
};
use reth_primitives::{Address, U256};
use rstest::*;
use starknet::core::types::FieldElement;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_eoa(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();

    // When
    let nonce = eth_provider.transaction_count(Address::zero(), None).await.unwrap();

    // Then
    // Zero address shouldn't throw 'ContractNotFound', but return zero
    assert_eq!(U256::from(0), nonce);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce_contract_account(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let counter_address: Felt252Wrapper = counter.evm_address.into();

    // When
    let nonce_initial = eth_provider.transaction_count(counter_address.try_into().unwrap(), None).await.unwrap();

    // Then
    assert_eq!(nonce_initial, U256::from(1));
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_eoa_balance(#[future] katana: Katana, _setup: ()) {
    // Given
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    // When
    let eoa_balance = eth_provider.balance(eoa.evm_address().unwrap(), None).await.unwrap();
    let eoa_balance = FieldElement::from_bytes_be(&eoa_balance.to_be_bytes()).unwrap();

    // Then
    assert!(eoa_balance > FieldElement::ZERO);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_storage_at(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();
    let counter_evm_address: Felt252Wrapper = counter.evm_address.into();
    let counter_evm_address = counter_evm_address.try_into().expect("Failed to convert EVM address");

    // When
    eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Then
    let count = eth_provider.storage_at(counter_evm_address, U256::from(0), None).await.unwrap();
    assert_eq!(U256::from(1), count);
}
