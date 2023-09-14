use kakarot_rpc_core::client::api::KakarotStarknetApi;
use kakarot_rpc_core::client::constants::DEPLOY_FEE;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_test_utils::constants::EOA_RECEIVER_ADDRESS;
use kakarot_test_utils::deploy_helpers::KakarotTestEnvironmentContext;
use kakarot_test_utils::execution_helpers::execute_eth_transfer_tx;
use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
use rstest::*;
use starknet::core::types::{FieldElement, MaybePendingTransactionReceipt, TransactionReceipt, TransactionStatus};
use starknet::providers::Provider;

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_wait_for_confirmation_on_l2(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    let (client, kakarot) = kakarot_test_env_ctx.resources();
    let amount = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

    let transaction_hash =
        execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, *EOA_RECEIVER_ADDRESS, amount).await;
    let transaction_hash: FieldElement = Felt252Wrapper::try_from(transaction_hash).unwrap().into();

    let _ = client.wait_for_confirmation_on_l2(transaction_hash).await;

    let transaction_receipt = client.starknet_provider().get_transaction_receipt(transaction_hash).await.unwrap();

    match transaction_receipt {
        MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
            assert_eq!(TransactionStatus::AcceptedOnL2, receipt.status)
        }
        _ => panic!(
            "Expected MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke), got {:?}",
            transaction_receipt
        ),
    }
}
