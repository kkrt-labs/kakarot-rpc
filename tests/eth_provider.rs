#![cfg(feature = "testing")]
use std::str::FromStr as _;

use kakarot_rpc::eth_provider::provider::EthereumProvider;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::test_utils::eoa::Eoa as _;
use kakarot_rpc::test_utils::fixtures::{counter, katana, setup};
use kakarot_rpc::test_utils::{evm_contract::KakarotEvmContract, sequencer::Katana};
use reth_rpc_types::{CallInput, CallRequest};
use rstest::*;

use reth_primitives::{BlockNumberOrTag, Bytes, U256};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let eth_provider = katana.eth_provider();
    let eoa = katana.eoa();

    // Check nonce of Eoa
    let nonce_before = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();

    eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Check nonce of Eoa
    let nonce_after = eth_provider.transaction_count(eoa.evm_address().unwrap(), None).await.unwrap();
    assert_eq!(nonce_before + U256::from(1), nonce_after);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_fee_history(#[future] katana: Katana, _setup: ()) {
    let eth_provider = katana.eth_provider();

    let newest_block = eth_provider.block_number().await.unwrap().as_u64();
    let block_count = newest_block + 1;

    // Check fee history
    let fee_history =
        eth_provider.fee_history(U256::from(block_count), BlockNumberOrTag::Number(newest_block), None).await.unwrap();
    assert_eq!(fee_history.base_fee_per_gas.len(), block_count as usize);
    assert_eq!(fee_history.gas_used_ratio.len(), block_count as usize);
    assert_eq!(fee_history.oldest_block, U256::ZERO);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_estimate_gas(#[future] counter: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let eoa = counter.0.eoa();
    let eth_provider = counter.0.eth_provider();
    let counter = counter.1;

    let chain_id = eth_provider.chain_id().await.unwrap().unwrap_or_default();
    let counter_address: Felt252Wrapper = counter.evm_address.into();

    let request = CallRequest {
        from: Some(eoa.evm_address().unwrap()),
        to: Some(counter_address.try_into().unwrap()),
        input: CallInput { input: None, data: Some(Bytes::from_str("0x371303c0").unwrap()) }, // selector of "function inc()"
        chain_id: Some(chain_id),
        ..Default::default()
    };

    // When
    let estimate = eth_provider.estimate_gas(request, None).await.unwrap();

    // Then
    assert!(estimate > U256::from(0));
}
