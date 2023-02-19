use kakarot_rpc_core::{
    client::{
        constants::CHAIN_ID,
        types::{Block, BlockTransactions},
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

#[derive(Serialize, Deserialize)]
struct StarknetBlockTest {
    block_hash: String,
    block_number: u64,
    new_root: String,
    parent_hash: String,
    sequencer_address: String,
    status: String,
    timestamp: u64,
}

pub fn assert_block(block: Block, starknet_res: String, starknet_txs: String, hydrated: bool) {
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    assert_eq!(block.total_difficulty, U256::ZERO);
    assert_eq!(block.uncles, vec![]);
    assert_eq!(block.size, None);
    assert_eq!(block.base_fee_per_gas, Some(U256::from(16)));

    if hydrated {
        let starknet_block_hash =
            FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();

        let starknet_txs: Vec<StarknetTransaction> =
            serde_json::from_str::<Vec<StarknetTransaction>>(&starknet_txs).unwrap();
        let transactions = block.transactions;

        match transactions {
            BlockTransactions::Full(transactions) => {
                if let Some(first_tx) = transactions.first() {
                    assert_eq!(
                        first_tx.block_number,
                        Some(U256::from(starknet_data.block_number))
                    );
                    assert_eq!(
                        first_tx.block_hash,
                        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
                    );

                    assert_eq!(first_tx.chain_id, Some(CHAIN_ID.into()));
                    assert_eq!(first_tx.standard_v, U256::from(0));
                    assert_eq!(first_tx.creates, None);
                    assert_eq!(first_tx.access_list, None);
                    assert_eq!(first_tx.transaction_type, None);

                    assert_eq!(first_tx.to, None);
                    assert_eq!(first_tx.value, U256::from(100));
                    assert_eq!(first_tx.gas, U256::from(100));
                    assert_eq!(first_tx.gas_price, None);

                    if let Some(first_starknet_tx) = starknet_txs.first() {
                        match first_starknet_tx {
                            StarknetTransaction::Invoke(invoke_tx) => {
                                match invoke_tx {
                                    InvokeTransaction::V0(v0) => {
                                        assert_eq!(
                                            first_tx.hash,
                                            H256::from_slice(&v0.transaction_hash.to_bytes_be())
                                        );
                                        assert_eq!(
                                            first_tx.r,
                                            felt_option_to_u256(Some(&v0.signature[0])).unwrap()
                                        );
                                        assert_eq!(
                                            first_tx.s,
                                            felt_option_to_u256(Some(&v0.signature[1])).unwrap()
                                        );

                                        // nonce vaut None
                                        // from vaut None
                                    }
                                    InvokeTransaction::V1(v1) => {
                                        assert_eq!(
                                            first_tx.hash,
                                            H256::from_slice(&v1.transaction_hash.to_bytes_be())
                                        );

                                        // let starknet_nonce =
                                        // FieldElement::from_hex_be(&"0x34b".to_string()).unwrap();
                                        assert_eq!(first_tx.nonce, felt_to_u256(v1.nonce));
                                        assert_eq!(
                                            first_tx.from,
                                            starknet_address_to_ethereum_address(
                                                &v1.sender_address
                                            )
                                        );
                                        assert_eq!(
                                            first_tx.r,
                                            felt_option_to_u256(Some(&v1.signature[0])).unwrap()
                                        );
                                        assert_eq!(
                                            first_tx.s,
                                            felt_option_to_u256(Some(&v1.signature[1])).unwrap()
                                        );
                                    }
                                }
                            }
                            _ => {}
                        };
                    };

                    // let starknet_signature_r = FieldElement::from_str(
                    //     "0x5267c0d93467ddb5cfe0ab9db124ed5d57345e92a45111e7a08f8afa7666fae",
                    // )
                    // .unwrap();
                    // let starknet_signature_s = FieldElement::from_str(
                    //     "0x622c1e743ae1060293085a9702ea1c6a7f642eb47b8eb9fb51ca0d156c5f5dd",
                    // )
                    // .unwrap();
                    // assert_eq!(
                    //     first_tx.r,
                    //     felt_option_to_u256(Some(&starknet_signature_r)).unwrap()
                    // );
                    // assert_eq!(
                    //     first_tx.s,
                    //     felt_option_to_u256(Some(&starknet_signature_s)).unwrap()
                    // );

                    // TODO test first_tx.input
                }
            }
            _ => {}
        }
    } else {
        assert_eq!(block.transactions, BlockTransactions::Hashes(vec![]));
    }
}

pub fn assert_block_header(block: Block, starknet_res: String) {
    let header = block.header;
    let starknet_data = serde_json::from_str::<StarknetBlockTest>(&starknet_res).unwrap();

    let starknet_block_hash = FieldElement::from_str(starknet_data.block_hash.as_str()).unwrap();
    assert_eq!(
        header.hash,
        Some(H256::from_slice(&starknet_block_hash.to_bytes_be()))
    );
    assert_eq!(header.number, Some(U256::from(19612)));

    let starknet_parent_hash =
        FieldElement::from_str("0x137970a5417cf7d35eb4eeb04efe6312166f828eec76342338b0e3797ebf3c1")
            .unwrap();
    let parent_hash = H256::from_slice(&starknet_parent_hash.to_bytes_be());
    assert_eq!(header.parent_hash, parent_hash);
    assert_eq!(header.uncles_hash, parent_hash);

    let starknet_sequencer =
        FieldElement::from_str("0x5dcd266a80b8a5f29f04d779c6b166b80150c24f2180a75e82427242dab20a9")
            .unwrap();
    let sequencer = H160::from_slice(&starknet_sequencer.to_bytes_be()[12..32]);
    assert_eq!(header.author, sequencer);
    assert_eq!(header.miner, sequencer);

    let starknet_new_root =
        FieldElement::from_str("0x67cde84ecff30c4ca55cb46df37940df87a94cc416cb893eaa9fb4fb67ec513")
            .unwrap();
    let state_root = H256::from_slice(&starknet_new_root.to_bytes_be());
    assert_eq!(header.state_root, state_root);

    assert_eq!(
        header.transactions_root,
        H256::from_slice(
            &"0xac91334ba861cb94cba2b1fd63df7e87c15ca73666201abd10b5462255a5c642".as_bytes()[1..33],
        )
    );
    assert_eq!(
        header.receipts_root,
        H256::from_slice(
            &"0xf2c8755adf35e78ffa84999e48aba628e775bb7be3c70209738d736b67a9b549".as_bytes()[1..33],
        )
    );

    assert_eq!(header.extra_data, Bytes::from(b"0x00"));
    assert_eq!(header.logs_bloom, Bloom::default());
    assert_eq!(header.timestamp, U256::from(1675461581));

    assert_eq!(header.gas_used, U256::ZERO);
    assert_eq!(header.gas_limit, U256::from(1_000_000_000_000_u64));
    assert_eq!(header.difficulty, U256::ZERO);
    assert_eq!(header.size, None);
    assert_eq!(header.base_fee_per_gas, U256::from(10000));
    assert_eq!(header.mix_hash, H256::zero());
    assert_eq!(header.nonce, Some(H64::zero()));
}
