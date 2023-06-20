use std::collections::BTreeMap;

use async_trait::async_trait;
use reth_primitives::{Address, Bloom, Bytes, H256, H64, U256};
use reth_rpc_types::{Block, BlockTransactions, Header, Rich, RichBlock};
use serde::{Deserialize, Serialize};
use starknet::core::types::{MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs};

use super::client_api::KakarotClientError;
use super::convertible::ConvertibleStarknetBlock;
use super::helpers::starknet_address_to_ethereum_address;
use super::KakarotClient;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalance {
    pub contract_address: Address,
    pub token_balance: Option<U256>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenBalances {
    pub address: Address,
    pub token_balances: Vec<TokenBalance>,
}

pub struct BlockWithTxHashes(MaybePendingBlockWithTxHashes);

impl BlockWithTxHashes {
    pub fn new(block: MaybePendingBlockWithTxHashes) -> Self {
        Self(block)
    }
}

pub struct BlockWithTxs(MaybePendingBlockWithTxs);

impl BlockWithTxs {
    pub fn new(block: MaybePendingBlockWithTxs) -> Self {
        Self(block)
    }
}

#[async_trait]
impl ConvertibleStarknetBlock for BlockWithTxHashes {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, KakarotClientError> {
        // TODO: Fetch real data
        let gas_limit = U256::from(1_000_000);

        // TODO: Fetch real data
        let gas_used = U256::from(500_000);

        // TODO: Fetch real data
        let difficulty = U256::ZERO;

        // TODO: Fetch real data
        let nonce: Option<H64> = Some(H64::zero());

        // TODO: Fetch real data
        let size: Option<U256> = Some(U256::from(1_000_000));

        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data = Bytes::from(b"0x00");
        // TODO: Fetch real data
        let total_difficulty: U256 = U256::ZERO;
        // TODO: Fetch real data
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = H256::zero();

        match &self.0 {
            MaybePendingBlockWithTxHashes::PendingBlock(pending_block_with_tx_hashes) => {
                let parent_hash = H256::from_slice(&pending_block_with_tx_hashes.parent_hash.to_bytes_be());
                let sequencer = starknet_address_to_ethereum_address(&pending_block_with_tx_hashes.sequencer_address);
                let timestamp = U256::from_be_bytes(pending_block_with_tx_hashes.timestamp.to_be_bytes());

                // TODO: Add filter to tx_hashes
                let transactions = BlockTransactions::Hashes(
                    pending_block_with_tx_hashes
                        .transactions
                        .clone()
                        .into_iter()
                        .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                        .collect(),
                );

                let header = Header {
                    // PendingBlockWithTxHashes doesn't have a block hash
                    hash: None,
                    parent_hash,
                    uncles_hash: parent_hash,
                    author: sequencer,
                    miner: sequencer,
                    // PendingBlockWithTxHashes doesn't have a state root
                    state_root: H256::zero(),
                    // PendingBlockWithTxHashes doesn't have a transactions root
                    transactions_root: H256::zero(),
                    // PendingBlockWithTxHashes doesn't have a receipts root
                    receipts_root: H256::zero(),
                    // PendingBlockWithTxHashes doesn't have a block number
                    number: None,
                    gas_used,
                    gas_limit,
                    extra_data,
                    logs_bloom,
                    timestamp,
                    difficulty,
                    nonce,
                    size,
                    mix_hash,
                    withdrawals_root: Some(H256::zero()),
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: Some(base_fee_per_gas),
                    size,
                    withdrawals: Some(vec![]),
                };
                Ok(Rich::<Block> { inner: block, extra_info: BTreeMap::default() })
            }
            MaybePendingBlockWithTxHashes::Block(block_with_tx_hashes) => {
                let hash = H256::from_slice(&block_with_tx_hashes.block_hash.to_bytes_be());
                let parent_hash = H256::from_slice(&block_with_tx_hashes.parent_hash.to_bytes_be());

                let sequencer = starknet_address_to_ethereum_address(&block_with_tx_hashes.sequencer_address);

                let state_root = H256::zero();
                let number = U256::from(block_with_tx_hashes.block_number);
                let timestamp = U256::from(block_with_tx_hashes.timestamp);
                // TODO: Add filter to tx_hashes
                let transactions = BlockTransactions::Hashes(
                    block_with_tx_hashes
                        .transactions
                        .clone()
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
                    mix_hash,
                    withdrawals_root: Some(H256::zero()),
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: Some(base_fee_per_gas),
                    size,
                    withdrawals: Some(vec![]),
                };
                Ok(Rich::<Block> { inner: block, extra_info: BTreeMap::default() })
            }
        }
    }
}

#[async_trait]
impl ConvertibleStarknetBlock for BlockWithTxs {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, KakarotClientError> {
        // TODO: Fetch real data
        let gas_limit = U256::from(1_000_000);

        // TODO: Fetch real data
        let gas_used = U256::from(500_000);

        // TODO: Fetch real data
        let difficulty = U256::ZERO;

        // TODO: Fetch real data
        let nonce: Option<H64> = Some(H64::zero());

        // TODO: Fetch real data
        let size: Option<U256> = Some(U256::from(1_000_000));

        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data: Bytes = Bytes::from(b"0x00");
        // TODO: Fetch real data
        let total_difficulty: U256 = U256::ZERO;
        // TODO: Fetch real data
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = H256::zero();
        match &self.0 {
            MaybePendingBlockWithTxs::PendingBlock(pending_block_with_txs) => {
                let parent_hash = H256::from_slice(&pending_block_with_txs.parent_hash.to_bytes_be());

                let sequencer = starknet_address_to_ethereum_address(&pending_block_with_txs.sequencer_address);

                let timestamp = U256::from_be_bytes(pending_block_with_txs.timestamp.to_be_bytes());

                let transactions = client
                    .filter_starknet_into_eth_txs(pending_block_with_txs.transactions.clone(), None, None)
                    .await?;
                let header = Header {
                    // PendingBlockWithTxs doesn't have a block hash
                    hash: None,
                    parent_hash,
                    uncles_hash: parent_hash,
                    author: sequencer,
                    miner: sequencer,
                    // PendingBlockWithTxs doesn't have a state root
                    state_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a transactions root
                    transactions_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a receipts root
                    receipts_root: H256::zero(),
                    // PendingBlockWithTxs doesn't have a block number
                    number: None,
                    gas_used,
                    gas_limit,
                    extra_data,
                    logs_bloom,
                    timestamp,
                    difficulty,
                    nonce,
                    size,
                    mix_hash,
                    withdrawals_root: Some(H256::zero()),
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: Some(base_fee_per_gas),
                    size,
                    withdrawals: Some(vec![]),
                };
                Ok(Rich::<Block> { inner: block, extra_info: BTreeMap::default() })
            }
            MaybePendingBlockWithTxs::Block(block_with_txs) => {
                let hash = H256::from_slice(&block_with_txs.block_hash.to_bytes_be());
                let parent_hash = H256::from_slice(&block_with_txs.parent_hash.to_bytes_be());

                let sequencer = starknet_address_to_ethereum_address(&block_with_txs.sequencer_address);

                let state_root = H256::zero();
                let transactions_root = H256::zero();
                let receipts_root = H256::zero();

                let number = U256::from(block_with_txs.block_number);
                let timestamp = U256::from(block_with_txs.timestamp);

                let blockhash_opt = Some(H256::from_slice(&(block_with_txs.block_hash).to_bytes_be()));
                let blocknum_opt = Some(U256::from(block_with_txs.block_number));

                let transactions = client
                    .filter_starknet_into_eth_txs(block_with_txs.transactions.clone(), blockhash_opt, blocknum_opt)
                    .await?;

                let header = Header {
                    hash: Some(hash),
                    parent_hash,
                    uncles_hash: parent_hash,
                    author: sequencer,
                    miner: sequencer,
                    state_root,
                    // BlockWithTxHashes doesn't have a transactions root
                    transactions_root,
                    // BlockWithTxHashes doesn't have a receipts root
                    receipts_root,
                    number: Some(number),
                    gas_used,
                    gas_limit,
                    extra_data,
                    logs_bloom,
                    timestamp,
                    difficulty,
                    nonce,
                    size,
                    mix_hash,
                    withdrawals_root: Some(H256::zero()),
                };
                let block = Block {
                    header,
                    total_difficulty,
                    uncles: vec![],
                    transactions,
                    base_fee_per_gas: Some(base_fee_per_gas),
                    size,
                    withdrawals: Some(vec![]),
                };
                Ok(Rich::<Block> { inner: block, extra_info: BTreeMap::default() })
            }
        }
    }
}
