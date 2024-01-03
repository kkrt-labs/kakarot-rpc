#[cfg(test)]
mod test_utils;
use kakarot_rpc::models::felt::Felt252Wrapper;
use rstest::*;
use starknet::providers::Provider;
use test_utils::eoa::Eoa;
use test_utils::evm_contract::KakarotEvmContract;
use test_utils::fixtures::{counter, katana};
use test_utils::sequencer::Katana;

use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_nonce(#[future] counter: (Katana, KakarotEvmContract)) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();

    // Check nonce of Eoa
    let nonce_before =
        client.nonce(eoa.evm_address().unwrap(), BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();

    eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Check nonce of Eoa
    let nonce_after =
        client.nonce(eoa.evm_address().unwrap(), BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();
    assert_eq!(nonce_before + U256::from(1), nonce_after);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_get_evm_address(#[future] counter: (Katana, KakarotEvmContract)) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();

    // Check counter EVM address
    let counter_evm_address = client.get_evm_address(&counter.starknet_address).await.unwrap();
    assert_eq!(Address::try_from(Felt252Wrapper::from(counter.evm_address)).unwrap(), counter_evm_address);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_fee_history(#[future] katana: Katana) {
    let client = katana.client();

    let newest_block = client.starknet_provider().block_number().await.unwrap();
    let block_count = newest_block + 1;

    // Check fee history
    let fee_history =
        client.fee_history(U256::from(block_count), BlockNumberOrTag::Number(newest_block), None).await.unwrap();
    assert_eq!(fee_history.base_fee_per_gas.len(), block_count as usize);
    assert_eq!(fee_history.gas_used_ratio.len(), block_count as usize);
    assert_eq!(fee_history.oldest_block, U256::ZERO);
}

#[tokio::test]
// Ignore until #649 is fixed
#[ignore]
async fn test_estimate_gas() {
    // // Given
    // let client = init_testnet_client();

    // let request = CallRequest {
    //     from: Some(*ACCOUNT_ADDRESS_EVM), // account address
    //     to: Some(*COUNTER_ADDRESS_EVM),   // counter address
    //     input: CallInput { input: None, data: Some(Bytes::from_str(INC_DATA).unwrap()) }, // call to inc()
    //     chain_id: Some(U64::from(CHAIN_ID)), // "KKRT" chain id
    //     ..Default::default()
    // };
    // let block_id = BlockId::Number(BlockNumberOrTag::Latest);

    // // When
    // let estimate = client.estimate_gas(request, block_id).await.unwrap();

    // // Then
    // assert!(estimate > U256::from(0));
}

#[tokio::test]
// Ignore until #649 is fixed and test runs against Madara
#[ignore]
async fn test_gas_price() {
    // // Given
    // let client = init_testnet_client();

    // // When
    // let gas_price = client.gas_price().await.unwrap();

    // // Then
    // assert!(gas_price > U256::from(0));
}
