use std::collections::BTreeMap;

use eyre::Result;
use reth_primitives::{
    rpc::{BlockId as EthBlockId, BlockNumber},
    Bloom, Bytes, H160, H256, H64, U256,
};

use reth_rpc_types::{Block, BlockTransactions, Header, Rich, RichBlock};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::models::{
        BlockId as StarknetBlockId, BlockTag, MaybePendingBlockWithTxHashes,
        MaybePendingBlockWithTxs,
    },
};

use crate::lightclient::LightClientError;
extern crate hex;

pub enum MaybePendingStarknetBlock {
    BlockWithTxHashes(MaybePendingBlockWithTxHashes),
    BlockWithTxs(MaybePendingBlockWithTxs),
}

pub fn ethers_block_id_to_starknet_block_id(
    block: EthBlockId,
) -> Result<StarknetBlockId, LightClientError> {
    match block {
        EthBlockId::Hash(hash) => {
            let address_hex = hex::encode(hash);
            let address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
                LightClientError::OtherError(anyhow::anyhow!(
                    "Failed to convert Starknet block hash to FieldElement: {}",
                    e
                ))
            })?;
            Ok(StarknetBlockId::Hash(address_felt))
        }
        EthBlockId::Number(number) => match number {
            BlockNumber::Latest => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Finalized => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Safe => Ok(StarknetBlockId::Tag(BlockTag::Latest)),
            BlockNumber::Earliest => Ok(StarknetBlockId::Number(0)),
            BlockNumber::Pending => Ok(StarknetBlockId::Tag(BlockTag::Pending)),
            BlockNumber::Number(num) => Ok(StarknetBlockId::Number(num.as_u64())),
        },
    }
}

pub fn starknet_block_to_eth_block(block: MaybePendingStarknetBlock) -> RichBlock {
    // Fixed fields in the Ethereum block as Starknet does not have these fields
    let gas_limit = U256::ZERO;
    let gas_used = U256::ZERO;
    let difficulty = U256::ZERO;
    let nonce: Option<H64> = None;
    let size: Option<U256> = None;
    // Bloom is a byte array of length 256
    let logs_bloom = Bloom::default();
    let extra_data = Bytes::from(b"0x00");
    let total_difficulty: U256 = U256::ZERO;

    match block {
        MaybePendingStarknetBlock::BlockWithTxHashes(maybe_pending_block) => {
            match maybe_pending_block {
                MaybePendingBlockWithTxHashes::PendingBlock(pending_block_with_tx_hashes) => {
                    let parent_hash =
                        H256::from_slice(&pending_block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &pending_block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let timestamp =
                        U256::from_be_bytes(pending_block_with_tx_hashes.timestamp.to_be_bytes());
                    let transactions = BlockTransactions::Hashes(
                        pending_block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                            .collect(),
                    );
                    let header = Header {
                        // PendingblockWithTxHashes doesn't have a block hash
                        hash: None,
                        parent_hash,
                        uncles_hash: parent_hash,
                        author: sequencer,
                        miner: sequencer,
                        // PendingblockWithTxHashes doesn't have a state root
                        state_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a transactions root
                        transactions_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a receipts root
                        receipts_root: H256::zero(),
                        // PendingblockWithTxHashes doesn't have a block number
                        number: None,
                        gas_used,
                        gas_limit,
                        extra_data,
                        logs_bloom,
                        timestamp,
                        difficulty,
                        nonce,
                        size,
                    };
                    let block = Block {
                        header,
                        total_difficulty,
                        uncles: vec![],
                        transactions,
                        base_fee_per_gas: None,
                        size,
                    };
                    Rich::<Block> {
                        inner: block,
                        extra_info: BTreeMap::default(),
                    }
                }
                MaybePendingBlockWithTxHashes::Block(block_with_tx_hashes) => {
                    let hash = H256::from_slice(&block_with_tx_hashes.block_hash.to_bytes_be());
                    let parent_hash =
                        H256::from_slice(&block_with_tx_hashes.parent_hash.to_bytes_be());
                    let sequencer = H160::from_slice(
                        &block_with_tx_hashes.sequencer_address.to_bytes_be()[12..32],
                    );
                    let state_root = H256::from_slice(&block_with_tx_hashes.new_root.to_bytes_be());
                    let number = U256::from(block_with_tx_hashes.block_number);
                    let timestamp = U256::from(block_with_tx_hashes.timestamp);
                    let transactions = BlockTransactions::Hashes(
                        block_with_tx_hashes
                            .transactions
                            .into_iter()
                            .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                            .collect(),
                    );
                    let header = Header {
                        hash: Some(hash),
                        parent_hash,
                        uncles_hash: parent_hash,
                        author: sequencer,
                        miner: sequencer,
                        state_root,
                        // BlockWithTxHashes doesn't have a transactions root
                        transactions_root: H256::zero(),
                        // BlockWithTxHashes doesn't have a receipts root
                        receipts_root: H256::zero(),
                        number: Some(number),
                        gas_used,
                        gas_limit,
                        extra_data,
                        logs_bloom,
                        timestamp,
                        difficulty,
                        nonce,
                        size,
                    };
                    let block = Block {
                        header,
                        total_difficulty,
                        uncles: vec![],
                        transactions,
                        base_fee_per_gas: None,
                        size,
                    };
                    Rich::<Block> {
                        inner: block,
                        extra_info: BTreeMap::default(),
                    }
                }
            }
        }
        MaybePendingStarknetBlock::BlockWithTxs(maybe_pending_block) => match maybe_pending_block {
            MaybePendingBlockWithTxs::PendingBlock(_pending_block_with_txs) => {
                unimplemented!("PendingBlockWithTxs is not supported yet")
            }
            MaybePendingBlockWithTxs::Block(_block_with_txs) => {
                unimplemented!("BlockWithTxs is not supported yet")
            }
        },
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use reth_primitives::rpc::{BlockId as EthBlockId, BlockNumber};
//     use starknet::providers::jsonrpc::models::BlockId as StarknetBlockId;
//     extern crate hex;

//     #[test]
//     fn test_ethers_block_id_to_starknet_block_id() {
//         let block_id = EthBlockId::Number(BlockNumber::Number(1.into()));
//         let starknet_block_id: StarknetBlockId =
//             ethers_block_id_to_starknet_block_id(block_id).unwrap();
//         assert_eq!(starknet_block_id, StarknetBlockId::Number(1));
//     }
// }
