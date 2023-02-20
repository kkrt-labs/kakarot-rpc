use kakarot_rpc_core::{
    client::{
        constants::CHAIN_ID,
        types::{Block, BlockTransactions, Transaction},
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

pub fn assert_block(block: Block, starknet_res: String, starknet_txs: String, hydrated: bool) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    assert_eq!(block.total_difficulty, U256::ZERO);
    assert_eq!(block.uncles, vec![]);
    assert_eq!(block.size, None);
    assert_eq!(block.base_fee_per_gas, Some(U256::from(16)));

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();
    let res_transactions = block.transactions;
    if hydrated {
        let starknet_txs = serde_json::from_str::<BlockTransactionObj>(&starknet_txs).unwrap();

        match res_transactions {
            BlockTransactions::Full(transactions) => {
                for (i, transaction) in transactions.into_iter().enumerate() {
                    assert_eq!(
                        transaction.block_number,
                        Some(U256::from(starknet_data.block_number))
                    );
                    assert_eq!(
                        transaction.block_hash,
                        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
                    );

                    assert_transaction(transaction.clone(), starknet_txs.transactions[i].clone());
                }
            }
            BlockTransactions::Hashes(_) => {
                panic!("BlockTransactions::Hashes should not be returned")
            }
        }
    } else {
        let starknet_tx_hashes =
            serde_json::from_str::<BlockTransactionHashesObj>(&starknet_txs).unwrap();

        match res_transactions {
            BlockTransactions::Hashes(transactions) => {
                for (i, transaction) in transactions.into_iter().enumerate() {
                    assert_eq!(
                        transaction,
                        H256::from_slice(&starknet_tx_hashes.transactions[i].to_bytes_be())
                    );
                }
            }
            BlockTransactions::Full(_) => {
                panic!("BlockTransactions::Full should not be returned")
            }
        }
    }
}

pub fn assert_block_header(block: Block, starknet_res: String, hydrated: bool) {
    let header = block.header;
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();
    assert_eq!(
        header.hash,
        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
    );
    assert_eq!(header.number, Some(U256::from(starknet_data.block_number)));

    let starknet_parent_hash = FieldElement::from_str(starknet_data.parent_hash.as_str()).unwrap();
    let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
    assert_eq!(header.parent_hash, parent_hash);
    assert_eq!(header.uncles_hash, parent_hash);

    let starknet_sequencer =
        FieldElement::from_str(starknet_data.sequencer_address.as_str()).unwrap();
    let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
    assert_eq!(header.author, sequencer);
    assert_eq!(header.miner, sequencer);

    let starknet_new_root = FieldElement::from_str(starknet_data.new_root.as_str()).unwrap();
    let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
    assert_eq!(header.state_root, state_root);

    assert_eq!(header.timestamp, U256::from(starknet_data.timestamp));

    if hydrated {
        assert_eq!(
            header.transactions_root,
            H256::from_slice(
                &"0xac91334ba861cb94cba2b1fd63df7e87c15ca73666201abd10b5462255a5c642".as_bytes()
                    [1..33],
            )
        );
        assert_eq!(
            header.receipts_root,
            H256::from_slice(
                &"0xf2c8755adf35e78ffa84999e48aba628e775bb7be3c70209738d736b67a9b549".as_bytes()
                    [1..33],
            )
        );
    } else {
        assert_eq!(header.transactions_root, H256::zero());
        assert_eq!(header.receipts_root, H256::zero());
    };

    assert_eq!(header.extra_data, Bytes::from(b"0x00"));
    assert_eq!(header.logs_bloom, Bloom::default());

    assert_eq!(header.gas_used, U256::ZERO);
    assert_eq!(header.gas_limit, U256::from(1_000_000_000_000_u64));
    assert_eq!(header.difficulty, U256::ZERO);
    assert_eq!(header.size, None);
    assert_eq!(header.base_fee_per_gas, U256::from(10000));
    assert_eq!(header.mix_hash, H256::zero());
    assert_eq!(header.nonce, Some(H64::zero()));
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
                        starknet_address_to_ethereum_address(&v1.sender_address)
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
