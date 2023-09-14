use kakarot_rpc_core::client::api::KakarotEthApi;
use kakarot_rpc_core::client::constants::TX_ORIGIN_ZERO;
use kakarot_test_utils::deploy_helpers::KakarotTestEnvironmentContext;
use kakarot_test_utils::execution_helpers::execute_eth_tx;
use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
use reth_primitives::BlockId;
use rstest::*;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_counter(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    // Given
    let (client, _, counter, counter_eth_address) = kakarot_test_env_ctx.resources_with_contract("Counter");

    // When
    let hash = execute_eth_tx(&kakarot_test_env_ctx, "Counter", "inc", vec![]).await;
    client.transaction_receipt(hash).await.expect("increment transaction failed");

    let count_selector = counter.abi.function("count").unwrap().short_signature();
    let counter_bytes = client
        .call(
            *TX_ORIGIN_ZERO,
            counter_eth_address,
            count_selector.into(),
            BlockId::Number(reth_primitives::BlockNumberOrTag::Latest),
        )
        .await
        .unwrap();

    let num = *counter_bytes.last().expect("Empty byte array");

    // Then
    assert_eq!(num, 1);
}
