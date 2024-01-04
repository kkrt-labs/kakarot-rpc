#[cfg(test)]
mod test_utils;

use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::models::transaction::transaction::StarknetTransaction;
use reth_primitives::U256;
use rstest::*;
use starknet::core::types::{BlockId, MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::providers::Provider;
use test_utils::evm_contract::KakarotEvmContract;
use test_utils::fixtures::counter;
use test_utils::sequencer::Katana;

use crate::test_utils::eoa::KakarotEOA;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_is_kakarot_tx(#[future] counter: (Katana, KakarotEvmContract)) {
    // Increment a counter
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();
    let starknet_tx_hash =
        KakarotEOA::call_evm_contract(eoa, &counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Query transaction
    let tx = client
        .starknet_provider()
        .get_transaction_by_hash(starknet_tx_hash)
        .await
        .expect("Failed to query transaction");
    let starknet_tx: StarknetTransaction = tx.into();
    let is_kakarot_tx = starknet_tx.is_kakarot_tx(client).await.unwrap();

    // Then
    assert!(is_kakarot_tx);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_transaction(#[future] counter: (Katana, KakarotEvmContract)) {
    // Increment a counter
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();
    let starknet_tx_hash = eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    // Query transaction
    let tx = client
        .starknet_provider()
        .get_transaction_by_hash(starknet_tx_hash)
        .await
        .expect("Failed to query transaction");
    let starknet_tx: StarknetTransaction = tx.into();

    // Get additional tx information block_number, block_hash, transaction_index
    let tx_receipt = client
        .starknet_provider()
        .get_transaction_receipt(starknet_tx_hash)
        .await
        .expect("Failed to query transaction receipt");
    let (block_number, block_hash, transaction_index) = match tx_receipt {
        MaybePendingTransactionReceipt::Receipt(tx_receipt) => {
            if let TransactionReceipt::Invoke(tx_receipt) = tx_receipt {
                let block_number = tx_receipt.block_number;
                let block_hash = tx_receipt.block_hash;
                let transaction_index = client
                    .starknet_provider()
                    .get_block_with_tx_hashes(BlockId::Hash(block_hash))
                    .await
                    .unwrap()
                    .transactions()
                    .binary_search(&starknet_tx_hash)
                    .unwrap();
                (
                    Some(U256::from(block_number)),
                    Some(Felt252Wrapper::from(block_hash).into()),
                    Some(U256::from(transaction_index)),
                )
            } else {
                panic!("Transaction receipt not found or not invoke")
            }
        }
        MaybePendingTransactionReceipt::PendingReceipt(_) => (None, None, None),
    };
    let _eth_tx = starknet_tx.to_eth_transaction(client, block_hash, block_number, transaction_index).await.unwrap();

    // TODO: Assert that the transaction is valid
}
