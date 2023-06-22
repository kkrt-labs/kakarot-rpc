use std::str::FromStr;

use reth_primitives::{Bloom, Bytes, H160, H256, U128, U256};
use reth_rpc_types::{Block, BlockTransactions, Rich, Signature, Transaction};
use serde::{Deserialize, Serialize};
use starknet::core::types::{FieldElement, InvokeTransaction, Transaction as StarknetTransaction};

use crate::client::constants::gas::BASE_FEE_PER_GAS;
use crate::client::constants::{CHAIN_ID, DIFFICULTY, GAS_LIMIT, GAS_USED, MIX_HASH, NONCE, SIZE, TOTAL_DIFFICULTY};
use crate::client::helpers::{felt_option_to_u256, felt_to_u256, starknet_address_to_ethereum_address};

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

pub fn assert_block(block: &Rich<Block>, starknet_res: String, starknet_txs: String, hydrated: bool) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    assert_eq!(block.total_difficulty, *TOTAL_DIFFICULTY);
    assert_eq!(block.uncles, vec![]);
    assert_eq!(block.size, *SIZE);

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();

    if hydrated {
        let starknet_txs = serde_json::from_str::<BlockTransactionObj>(&starknet_txs).unwrap();
        if let BlockTransactions::Full(transactions) = block.transactions.clone() {
            for (i, transaction) in starknet_txs.transactions.into_iter().enumerate() {
                assert_eq!(transactions[i].block_number, Some(U256::from(starknet_data.block_number)));
                assert_eq!(transactions[i].block_hash, Some(H256::from_slice(&starknet_block_hash.to_bytes_be())));

                assert_transaction(transactions[i].clone(), transaction.clone());
            }
        } else {
            panic!("BlockTransactions::Hashes should not be returned")
        }
    } else {
        let starknet_tx_hashes = serde_json::from_str::<BlockTransactionHashesObj>(&starknet_txs).unwrap();

        if let BlockTransactions::Hashes(transactions) = block.transactions.clone() {
            for (i, transaction) in starknet_tx_hashes.transactions.into_iter().enumerate() {
                assert_eq!(transactions[i], H256::from_slice(&transaction.to_bytes_be()));
            }
        } else {
            panic!("BlockTransactions::Full should not be returned")
        }
    }
}

pub fn assert_block_header(block: &Rich<Block>, starknet_res: String, hydrated: bool) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();
    assert_eq!(block.header.hash, Some(H256::from_slice(&starknet_block_hash.to_bytes_be())));
    assert_eq!(block.header.number, Some(U256::from(starknet_data.block_number)));

    let starknet_parent_hash = FieldElement::from_str(starknet_data.parent_hash.as_str()).unwrap();
    let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
    assert_eq!(block.header.parent_hash, parent_hash);
    assert_eq!(block.header.uncles_hash, parent_hash);

    let starknet_sequencer = FieldElement::from_str(starknet_data.sequencer_address.as_str()).unwrap();
    let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
    assert_eq!(block.header.miner, sequencer);

    let starknet_new_root = FieldElement::from_str(starknet_data.new_root.as_str()).unwrap();
    let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
    assert_eq!(block.header.state_root, state_root);

    assert_eq!(block.header.timestamp, U256::from(starknet_data.timestamp));

    if hydrated {
        assert_eq!(
            block.header.transactions_root,
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap()
        );
        assert_eq!(
            block.header.receipts_root,
            H256::from_str("0x0000000000000000000000000000000000000000000000000000000000000000").unwrap()
        );
    } else {
        assert_eq!(block.header.transactions_root, H256::zero());
        assert_eq!(block.header.receipts_root, H256::zero());
    };

    assert_eq!(block.header.extra_data, Bytes::from(b"0x00"));
    assert_eq!(block.header.logs_bloom, Bloom::default());

    assert_eq!(block.header.gas_used, *GAS_USED);
    assert_eq!(block.header.gas_limit, *GAS_LIMIT);
    assert_eq!(block.header.difficulty, *DIFFICULTY);
    assert_eq!(block.header.base_fee_per_gas, Some(U256::from(BASE_FEE_PER_GAS)));
    assert_eq!(block.header.mix_hash, *MIX_HASH);
    assert_eq!(block.header.nonce, *NONCE);
}

pub fn assert_transaction(ether_tx: Transaction, starknet_tx: StarknetTransaction) {
    assert_eq!(ether_tx.chain_id, Some(CHAIN_ID.into()));
    assert_eq!(ether_tx.access_list, None);
    assert_eq!(ether_tx.transaction_type, None);

    assert_eq!(ether_tx.to, None);
    assert_eq!(ether_tx.value, U256::from(100));
    assert_eq!(ether_tx.gas, U256::from(100));
    assert_eq!(ether_tx.gas_price, None);
    let index = match ether_tx.transaction_index {
        Some(_) => Some(U256::from(0)),
        _ => None,
    };
    assert_eq!(ether_tx.transaction_index, index);
    assert_eq!(ether_tx.max_fee_per_gas, None);
    assert_eq!(ether_tx.max_priority_fee_per_gas, Some(U128::ZERO));

    match starknet_tx {
        StarknetTransaction::Invoke(invoke_tx) => {
            match invoke_tx {
                InvokeTransaction::V0(v0) => {
                    assert_eq!(ether_tx.hash, H256::from_slice(&v0.transaction_hash.to_bytes_be()));
                    assert_eq!(ether_tx.nonce, felt_to_u256(v0.nonce));
                    assert_eq!(ether_tx.from, starknet_address_to_ethereum_address(&v0.contract_address));
                    // r and s values are extracted from the calldata of the first transaction
                    // in the starknet_getBlockWithTxs.json file.
                    // v value is calculated from the parity of the y coordinate of the signature,
                    // to which we add 35 + 2 * CHAIN_ID (based on https://eips.ethereum.org/EIPS/eip-155).
                    let signature = Signature {
                        r: U256::from_str("0x05e6a35e537e8d99c81bf2d4e7e8a410e7f6f3f8b1f07edc28bf226d3ac2cae12")
                            .unwrap(),
                        s: U256::from_str("0x01910d7b4784e7347a6c7dccf8b8051c06f091347eb4a4a2f6092f1541cb62de7")
                            .unwrap(),
                        v: U256::from_str("0x000000000000000000000000000000000000000000000000000000009696a4cc")
                            .unwrap(),
                    };
                    assert_eq!(ether_tx.signature, Some(signature));
                }
                InvokeTransaction::V1(v1) => {
                    assert_eq!(ether_tx.hash, H256::from_slice(&v1.transaction_hash.to_bytes_be()));
                    assert_eq!(ether_tx.nonce, felt_to_u256(v1.nonce));
                    assert_eq!(ether_tx.from, H160::from_str("0x54b288676b749def5fc10eb17244fe2c87375de1").unwrap());
                    // r and s values are extracted from the calldata of the first transaction
                    // in the starknet_getBlockWithTxs.json file.
                    // v value is calculated from the parity of the y coordinate of the signature,
                    // to which we add 35 + 2 * CHAIN_ID (based on https://eips.ethereum.org/EIPS/eip-155).
                    let signature = Signature {
                        r: U256::from_str("0x05e6a35e537e8d99c81bf2d4e7e8a410e7f6f3f8b1f07edc28bf226d3ac2cae12")
                            .unwrap(),
                        s: U256::from_str("0x01910d7b4784e7347a6c7dccf8b8051c06f091347eb4a4a2f6092f1541cb62de7")
                            .unwrap(),
                        v: U256::from_str("0x000000000000000000000000000000000000000000000000000000009696a4cc")
                            .unwrap(),
                    };
                    assert_eq!(ether_tx.signature, Some(signature));
                    // TODO: test ether_tx.input
                }
            }
        }
        StarknetTransaction::Deploy(deploy_tx) => {
            assert_eq!(ether_tx.hash, H256::from_slice(&deploy_tx.transaction_hash.to_bytes_be()));
            // TODO: nonce, from, input, v, r, s
        }
        StarknetTransaction::DeployAccount(deploy_account_tx) => {
            assert_eq!(ether_tx.hash, H256::from_slice(&deploy_account_tx.transaction_hash.to_bytes_be()));
            assert_eq!(ether_tx.nonce, felt_to_u256(deploy_account_tx.nonce));
            let signature = Signature {
                v: felt_option_to_u256(Some(&deploy_account_tx.signature[2])).unwrap(),
                r: felt_option_to_u256(Some(&deploy_account_tx.signature[0])).unwrap(),
                s: felt_option_to_u256(Some(&deploy_account_tx.signature[1])).unwrap(),
            };
            assert_eq!(ether_tx.signature, Some(signature));
            // TODO: from
        }
        StarknetTransaction::L1Handler(_) | StarknetTransaction::Declare(_) => {
            // L1Handler & Declare transactions not supported for now in Kakarot
            todo!();
        }
    };
}
