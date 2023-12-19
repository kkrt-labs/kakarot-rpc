mod test_utils;
use kakarot_rpc::models::block::{BlockWithTxHashes, BlockWithTxs};
use kakarot_rpc::models::felt::Felt252Wrapper;
use reth_primitives::U256;
use reth_rpc_types::BlockTransactions;
use rstest::*;
use starknet::core::types::{BlockId, MaybePendingTransactionReceipt, TransactionReceipt};
use starknet::providers::Provider;
use test_utils::evm_contract::KakarotEvmContract;
use test_utils::fixtures::counter;
use test_utils::sequencer::Katana;

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_block_with_tx_hashes(#[future] counter: (Katana, KakarotEvmContract)) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();
    let starknet_tx_hash = eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    let tx_receipt = client
        .starknet_provider()
        .get_transaction_receipt(starknet_tx_hash)
        .await
        .expect("Failed to query transaction receipt");
    let block_number = match tx_receipt {
        MaybePendingTransactionReceipt::Receipt(tx_receipt) => {
            if let TransactionReceipt::Invoke(tx_receipt) = tx_receipt {
                tx_receipt.block_number
            } else {
                panic!("Transaction receipt is not an invoke transaction");
            }
        }
        MaybePendingTransactionReceipt::PendingReceipt(_) => panic!("Transaction receipt is pending"),
    };

    let block: BlockWithTxHashes = client
        .starknet_provider()
        .get_block_with_tx_hashes(BlockId::Number(block_number))
        .await
        .expect("Failed to query block")
        .into();

    let eth_block = block.to_eth_block(client).await.inner;

    // TODO: Check that the block is valid
    assert_eq!(&eth_block.header.number.unwrap(), &U256::from(block_number));
    let tx_hashes = match eth_block.transactions {
        BlockTransactions::Hashes(tx_hashes) => tx_hashes,
        _ => panic!("Expected block transactions to be hashes"),
    };
    assert_eq!(tx_hashes.len(), 1);
    let tx_hash = Felt252Wrapper::from(starknet_tx_hash).into();
    assert_eq!(tx_hashes[0], tx_hash);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_to_eth_block_with_txs(#[future] counter: (Katana, KakarotEvmContract)) {
    let katana: Katana = counter.0;
    let counter = counter.1;
    let client = katana.client();
    let eoa = katana.eoa();
    let starknet_tx_hash = eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

    let tx_receipt = client
        .starknet_provider()
        .get_transaction_receipt(starknet_tx_hash)
        .await
        .expect("Failed to query transaction receipt");
    let block_number = match tx_receipt {
        MaybePendingTransactionReceipt::Receipt(tx_receipt) => {
            if let TransactionReceipt::Invoke(tx_receipt) = tx_receipt {
                tx_receipt.block_number
            } else {
                panic!("Transaction receipt is not an invoke transaction");
            }
        }
        MaybePendingTransactionReceipt::PendingReceipt(_) => panic!("Transaction receipt is pending"),
    };

    let block: BlockWithTxs = client
        .starknet_provider()
        .get_block_with_txs(BlockId::Number(block_number))
        .await
        .expect("Failed to query block")
        .into();

    let eth_block = block.to_eth_block(client).await.inner;

    // TODO: Check that the block is valid
    assert_eq!(&eth_block.header.number.unwrap(), &U256::from(block_number));
    let txs = match eth_block.transactions {
        BlockTransactions::Full(txs) => txs,
        _ => panic!("Expected block transactions to be full"),
    };
    assert_eq!(txs.len(), 1);
    let tx_hash = Felt252Wrapper::from(starknet_tx_hash).into();
    assert_eq!(txs[0].hash, tx_hash);
}
