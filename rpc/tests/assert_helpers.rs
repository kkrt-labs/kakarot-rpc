use kakarot_rpc_core::{
    client::{
        constants::CHAIN_ID,
        types::{Block, BlockTransactions, Rich, Transaction},
    },
    helpers::{felt_option_to_u256, felt_to_u256, starknet_address_to_ethereum_address},
};
use reth_primitives::{Bloom, Bytes, H160, H256, H64, U256};
use serde::{Deserialize, Serialize};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::models::{InvokeTransaction, Transaction as StarknetTransaction},
};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
struct StarknetBlockTest {
    block_hash: String,
    block_number: u64,
    new_root: String,
    parent_hash: String,
    sequencer_address: String,
    status: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockTransactionObj {
    transactions: Vec<StarknetTransaction>,
}

#[derive(Serialize, Deserialize, Debug)]
struct BlockTransactionHashesObj {
    transactions: Vec<FieldElement>,
}

pub fn assert_block(
    block: &Rich<Block>,
    starknet_res: String,
    starknet_txs: String,
    hydrated: bool,
) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    assert_eq!(block.total_difficulty, U256::ZERO);
    assert_eq!(block.uncles, vec![]);
    assert_eq!(block.size, Some(U256::from(1_000_000)));
    assert_eq!(block.base_fee_per_gas, Some(U256::from(16)));

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();

    if hydrated {
        let starknet_txs = serde_json::from_str::<BlockTransactionObj>(&starknet_txs).unwrap();
        if let BlockTransactions::Full(transactions) = block.transactions.clone() {
            for (i, transaction) in starknet_txs.transactions.into_iter().enumerate() {
                assert_eq!(
                    transactions[i].block_number,
                    Some(U256::from(starknet_data.block_number))
                );
                assert_eq!(
                    transactions[i].block_hash,
                    Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
                );

                assert_transaction(transactions[i].clone(), transaction.clone());
            }
        } else {
            panic!("BlockTransactions::Hashes should not be returned")
        }
    } else {
        let starknet_tx_hashes =
            serde_json::from_str::<BlockTransactionHashesObj>(&starknet_txs).unwrap();

        if let BlockTransactions::Hashes(transactions) = block.transactions.clone() {
            for (i, transaction) in starknet_tx_hashes.transactions.into_iter().enumerate() {
                assert_eq!(
                    transactions[i],
                    H256::from_slice(&transaction.to_bytes_be())
                );
            }
        } else {
            panic!("BlockTransactions::Full should not be returned")
        }
    }
}

pub fn assert_block_header(block: &Rich<Block>, starknet_res: String, hydrated: bool) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();
    assert_eq!(
        block.header.hash,
        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
    );
    assert_eq!(
        block.header.number,
        Some(U256::from(starknet_data.block_number))
    );

    let starknet_parent_hash = FieldElement::from_str(starknet_data.parent_hash.as_str()).unwrap();
    let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
    assert_eq!(block.header.parent_hash, parent_hash);
    assert_eq!(block.header.uncles_hash, parent_hash);

    let starknet_sequencer =
        FieldElement::from_str(starknet_data.sequencer_address.as_str()).unwrap();
    let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
    assert_eq!(block.header.author, sequencer);
    assert_eq!(block.header.miner, sequencer);

    let starknet_new_root = FieldElement::from_str(starknet_data.new_root.as_str()).unwrap();
    let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
    assert_eq!(block.header.state_root, state_root);

    assert_eq!(block.header.timestamp, U256::from(starknet_data.timestamp));

    if hydrated {
        assert_eq!(
            block.header.transactions_root,
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap()
        );
        assert_eq!(
            block.header.receipts_root,
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000")
                .unwrap()
        );
    } else {
        assert_eq!(block.header.transactions_root, H256::zero());
        assert_eq!(block.header.receipts_root, H256::zero());
    };

    assert_eq!(block.header.extra_data, Bytes::from(b"0x00"));
    assert_eq!(block.header.logs_bloom, Bloom::default());

    assert_eq!(block.header.gas_used, U256::from(500_000));
    assert_eq!(block.header.gas_limit, U256::from(1_000_000));
    assert_eq!(block.header.difficulty, U256::ZERO);
    assert_eq!(block.header.size, Some(U256::from(1_000_000)));
    assert_eq!(block.header.base_fee_per_gas, U256::from(16));
    assert_eq!(block.header.mix_hash, H256::zero());
    assert_eq!(block.header.nonce, Some(H64::zero()));
}

pub fn assert_transaction(ether_tx: Transaction, starknet_tx: StarknetTransaction) {
    assert_eq!(ether_tx.chain_id, Some(CHAIN_ID.into()));
    assert_eq!(ether_tx.standard_v, U256::from(0));
    assert_eq!(ether_tx.creates, None);
    assert_eq!(ether_tx.access_list, None);
    assert_eq!(ether_tx.transaction_type, None);

    assert_eq!(ether_tx.to, None);
    assert_eq!(ether_tx.value, U256::from(100));
    assert_eq!(ether_tx.gas, U256::from(100));
    assert_eq!(ether_tx.gas_price, None);
    assert_eq!(ether_tx.public_key, None);
    assert_eq!(ether_tx.transaction_index, None);
    assert_eq!(ether_tx.max_fee_per_gas, None);
    assert_eq!(ether_tx.max_priority_fee_per_gas, None);
    assert_eq!(ether_tx.raw, Bytes::default());

    match starknet_tx {
        StarknetTransaction::Invoke(invoke_tx) => {
            match invoke_tx {
                InvokeTransaction::V0(v0) => {
                    assert_eq!(
                        ether_tx.hash,
                        H256::from_slice(&v0.transaction_hash.to_bytes_be())
                    );
                    assert_eq!(ether_tx.nonce, felt_to_u256(v0.nonce));
                    assert_eq!(
                        ether_tx.from,
                        starknet_address_to_ethereum_address(&v0.contract_address)
                    );
                    assert_eq!(
                        ether_tx.r,
                        felt_option_to_u256(Some(&v0.signature[0])).unwrap()
                    );
                    assert_eq!(
                        ether_tx.s,
                        felt_option_to_u256(Some(&v0.signature[1])).unwrap()
                    );
                }
                InvokeTransaction::V1(v1) => {
                    assert_eq!(
                        ether_tx.hash,
                        H256::from_slice(&v1.transaction_hash.to_bytes_be())
                    );
                    assert_eq!(ether_tx.nonce, felt_to_u256(v1.nonce));
                    assert_eq!(
                        ether_tx.from,
                        H160::from_str("0x9296be4959e56b5df2200dbfa30594504a7fed61").unwrap()
                    );
                    assert_eq!(
                        ether_tx.r,
                        felt_option_to_u256(Some(&v1.signature[0])).unwrap()
                    );
                    assert_eq!(
                        ether_tx.s,
                        felt_option_to_u256(Some(&v1.signature[1])).unwrap()
                    );
                    // TODO: test ether_tx.input
                }
            }
        }
        StarknetTransaction::Deploy(deploy_tx) => {
            assert_eq!(
                ether_tx.hash,
                H256::from_slice(&deploy_tx.transaction_hash.to_bytes_be())
            );
            // TODO: nonce, from, input, v, r, s
        }
        StarknetTransaction::DeployAccount(deploy_account_tx) => {
            assert_eq!(
                ether_tx.hash,
                H256::from_slice(&deploy_account_tx.transaction_hash.to_bytes_be())
            );
            assert_eq!(ether_tx.nonce, felt_to_u256(deploy_account_tx.nonce));
            assert_eq!(
                ether_tx.r,
                felt_option_to_u256(Some(&deploy_account_tx.signature[0])).unwrap()
            );
            assert_eq!(
                ether_tx.s,
                felt_option_to_u256(Some(&deploy_account_tx.signature[1])).unwrap()
            );
            // TODO: from
        }
        StarknetTransaction::L1Handler(_) | StarknetTransaction::Declare(_) => {
            // L1Handler & Declare transactions not supported for now in Kakarot
            todo!();
        }
    };
}
