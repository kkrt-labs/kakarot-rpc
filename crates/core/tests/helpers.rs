use kakarot_rpc_core::client::constants::DEPLOY_FEE;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_test_utils::fixtures::katana;
use kakarot_test_utils::sequencer::Katana;
use reth_primitives::Address;
use rstest::*;
use starknet::core::types::{ExecutionResult, FieldElement, MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::providers::Provider;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_wait_for_confirmation_on_l2(#[future] katana: Katana) {
    // Given
    let client = katana.client();
    let eoa = katana.eoa();
    let amount = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();
    let to = Address::from(123);

    let transaction_hash = eoa.transfer(to, amount).await.expect("Failed to transfer funds");
    let transaction_hash: FieldElement = Felt252Wrapper::try_from(transaction_hash).unwrap().into();

    let _ = client.wait_for_confirmation_on_l2(transaction_hash).await;

    let transaction_receipt = client.starknet_provider().get_transaction_receipt(transaction_hash).await.unwrap();

    match transaction_receipt {
        MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
            assert!(matches!(receipt.execution_result, ExecutionResult::Succeeded))
        }
        _ => panic!(
            "Expected MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke), got {:?}",
            transaction_receipt
        ),
    }
}
