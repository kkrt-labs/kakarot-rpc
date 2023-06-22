pub mod convertible;

use std::iter::once;

use async_trait::async_trait;
use convertible::{ConvertibleStarknetBlock, ConvertibleStarknetEvent};
use num_bigint::BigUint;
use reth_primitives::{Address, Bloom, Bytes, H256, H64, U256};
use reth_rpc_types::{Block, BlockTransactions, Header, Log, RichBlock};
use serde::{Deserialize, Serialize};
use starknet::core::types::{
    Event, FieldElement, MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs, Transaction,
};

use crate::client::client_api::{KakarotClient, KakarotClientError};
use crate::client::helpers::starknet_address_to_ethereum_address;

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

/// Implement getters for fields that are present in Starknet Blocks, both in pending and validated
/// state. For example, `parent_hash` is present in both `PendingBlock` and `Block`.
macro_rules! implement_starknet_block_getters {
    ($(($enum:ty, $field:ident, $field_type:ty)),*) => {
        $(pub fn $field(&self) -> $field_type {
            match &self.0 {
                <$enum>::PendingBlock(pending_block_with_tx_hashes) => {
                    pending_block_with_tx_hashes.$field.clone()
                }
                <$enum>::Block(block_with_tx_hashes) => {
                    block_with_tx_hashes.$field.clone()
                }
            }
        })*
    };
}

/// Implement getters for fields that are only present in Starknet Blocks that are not pending.
/// For example, `block_hash` is only present in `Block` and not in `PendingBlock`.
macro_rules! implement_starknet_block_getters_not_pending {
    ($(($enum:ty, $field:ident, $field_type:ty)),*) => {
        $(pub fn $field(&self) -> Option<$field_type> {
            match &self.0 {
                <$enum>::PendingBlock(_) => {
                    None
                }
                <$enum>::Block(block_with_txs) => {
                    Some(block_with_txs.$field.clone())
                }
            }
        })*
    };
}

pub struct BlockWithTxHashes(MaybePendingBlockWithTxHashes);

impl BlockWithTxHashes {
    pub fn new(block: MaybePendingBlockWithTxHashes) -> Self {
        Self(block)
    }

    implement_starknet_block_getters!(
        (MaybePendingBlockWithTxHashes, parent_hash, FieldElement),
        (MaybePendingBlockWithTxHashes, sequencer_address, FieldElement),
        (MaybePendingBlockWithTxHashes, timestamp, u64),
        (MaybePendingBlockWithTxHashes, transactions, Vec<FieldElement>)
    );

    implement_starknet_block_getters_not_pending!(
        (MaybePendingBlockWithTxHashes, block_hash, FieldElement),
        (MaybePendingBlockWithTxHashes, block_number, u64)
    );
}

pub struct BlockWithTxs(MaybePendingBlockWithTxs);

impl BlockWithTxs {
    pub fn new(block: MaybePendingBlockWithTxs) -> Self {
        Self(block)
    }

    implement_starknet_block_getters!(
        (MaybePendingBlockWithTxs, parent_hash, FieldElement),
        (MaybePendingBlockWithTxs, sequencer_address, FieldElement),
        (MaybePendingBlockWithTxs, timestamp, u64),
        (MaybePendingBlockWithTxs, transactions, Vec<Transaction>)
    );

    implement_starknet_block_getters_not_pending!(
        (MaybePendingBlockWithTxs, block_hash, FieldElement),
        (MaybePendingBlockWithTxs, block_number, u64)
    );
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
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = H256::zero();

        let parent_hash = H256::from_slice(&self.parent_hash().to_bytes_be());
        let sequencer = starknet_address_to_ethereum_address(&self.sequencer_address());
        let timestamp = U256::from(self.timestamp());

        let hash = self.block_hash().as_ref().map(|hash| H256::from_slice(&hash.to_bytes_be()));
        let number = self.block_number().map(U256::from);

        // TODO: Add filter to tx_hashes
        let transactions = BlockTransactions::Hashes(
            self.transactions().iter().map(|tx| H256::from_slice(&tx.to_bytes_be())).collect(),
        );

        let header = Header {
            // PendingBlockWithTxHashes doesn't have a block hash
            hash,
            parent_hash,
            uncles_hash: parent_hash,
            miner: sequencer,
            // PendingBlockWithTxHashes doesn't have a state root
            state_root: H256::zero(),
            // PendingBlockWithTxHashes doesn't have a transactions root
            transactions_root: H256::zero(),
            // PendingBlockWithTxHashes doesn't have a receipts root
            receipts_root: H256::zero(),
            // PendingBlockWithTxHashes doesn't have a block number
            number,
            gas_used,
            gas_limit,
            extra_data,
            logs_bloom,
            timestamp,
            difficulty,
            nonce,
            base_fee_per_gas: Some(base_fee_per_gas),
            mix_hash,
            withdrawals_root: Some(H256::zero()),
        };
        let block =
            Block { header, total_difficulty: None, uncles: vec![], transactions, size, withdrawals: Some(vec![]) };
        Ok(block.into())
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
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = H256::zero();

        let parent_hash = H256::from_slice(&self.parent_hash().to_bytes_be());

        let sequencer = starknet_address_to_ethereum_address(&self.sequencer_address());

        let timestamp = U256::from(self.timestamp());

        let hash = self.block_hash().as_ref().map(|hash| H256::from_slice(&hash.to_bytes_be()));
        let number = self.block_number().map(U256::from);

        let transactions = client.filter_starknet_into_eth_txs(self.transactions(), hash, number).await?;
        let header = Header {
            // PendingBlockWithTxs doesn't have a block hash
            hash,
            parent_hash,
            uncles_hash: parent_hash,
            miner: sequencer,
            // PendingBlockWithTxs doesn't have a state root
            state_root: H256::zero(),
            // PendingBlockWithTxs doesn't have a transactions root
            transactions_root: H256::zero(),
            // PendingBlockWithTxs doesn't have a receipts root
            receipts_root: H256::zero(),
            // PendingBlockWithTxs doesn't have a block number
            number,
            gas_used,
            gas_limit,
            extra_data,
            logs_bloom,
            timestamp,
            difficulty,
            nonce,
            base_fee_per_gas: Some(base_fee_per_gas),
            mix_hash,
            withdrawals_root: Some(H256::zero()),
        };
        let block =
            Block { header, total_difficulty: None, uncles: vec![], transactions, size, withdrawals: Some(vec![]) };
        Ok(block.into())
    }
}

pub struct StarknetEvent(Event);

impl StarknetEvent {
    pub fn new(sn_event: Event) -> Self {
        Self(sn_event)
    }
}

#[async_trait]
impl ConvertibleStarknetEvent for StarknetEvent {
    async fn to_eth_log(&self, _client: &dyn KakarotClient) -> Result<Log, KakarotClientError> {
        // let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);
        // let address = client.get_evm_address(&self.0.from_address, &starknet_block_id).await.unwrap();
        // Convert the StarkNet address to a byte array
        let sn_bytes = self.0.from_address.clone().to_bytes_be();
        // Take the last 20 bytes of the StarkNet address to create the Ethereum address
        let address = Address::from_slice(&sn_bytes[12..32]);
        let topics: Vec<H256> = self.0.keys
        // Splits the keys into chunks of 2 elements each.
            .chunks(2)
            .map(|chunk| {
                // Converts the first element of the chunk to bytes and then to a BigUint.
                // BigUint gives us arbitrary precision, to avoid prime field wrap arounds
                let low = BigUint::from_bytes_be(&chunk[0].to_bytes_be());
                // If there is a second element in the chunk, it's converted to bytes and then to a BigUint. 
                // Otherwise, a BigUint is created from 0u128.
                let high = chunk.get(1).map_or(BigUint::from(0u128), |h| BigUint::from_bytes_be(&h.to_bytes_be()));
                // Calculates the result as `low + 2^128 * high`
                let result = low + (BigUint::from(2u128).pow(128u32) * high);
                // Converts the result to bytes.
                let bytes = result.to_bytes_be();
                // If the length of bytes is less than 32, prepends it with zeros to make it 32 bytes long.
                let bytes = once(0u8).cycle().take(32 - bytes.len()).chain(bytes.into_iter()).collect::<Vec<_>>();
                // Converts the bytes to H256 (32-byte hash).
                H256::from_slice(&bytes)
            })
            .collect(); // Collects all H256 hashes into a vector.

        let data: Bytes = self.0.data.iter()
            .flat_map(|felt| felt.to_bytes_be())
            .collect::<Vec<u8>>() // Collect into Vec<u8> first
            .into(); // Then convert into Bytes

        Ok(Log {
            address,
            topics,
            data,
            transaction_hash: Option::None,
            block_hash: Option::None,
            block_number: Option::None,
            log_index: Option::None,
            transaction_index: Option::None,
            removed: false,
        })
    }
}
