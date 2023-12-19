#[cfg(test)]
mod test_utils;

use crate::test_utils::evm_contract::KakarotEvmContract;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::models::transaction_receipt::StarknetTransactionReceipt;
use rstest::*;
use starknet::providers::Provider;
use test_utils::fixtures::counter;
use test_utils::sequencer::Katana;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_transaction_receipt(#[future] counter: (Katana, KakarotEvmContract)) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();
    let starknet_tx_hash = eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Get additional tx information block_number, block_hash, transaction_index
    let tx_receipt = client
        .starknet_provider()
        .get_transaction_receipt(starknet_tx_hash)
        .await
        .expect("Failed to query transaction receipt");

    let starknet_transaction_receipt: StarknetTransactionReceipt = tx_receipt.into();
    let eth_transaction_receipt =
        starknet_transaction_receipt.to_eth_transaction_receipt(client).await.unwrap().unwrap();

    // TODO: Assert that the transaction receipt is valid
    assert_eq!(eth_transaction_receipt.transaction_hash.unwrap(), Felt252Wrapper::from(starknet_tx_hash).into());
}
