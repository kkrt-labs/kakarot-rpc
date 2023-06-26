pub mod convertible;

use async_trait::async_trait;
use convertible::ConvertibleStarknetBlock;
use reth_primitives::{Address, Bloom, Bytes, H256, H64, U256};
use reth_rpc_types::{Block, BlockTransactions, Header, RichBlock, Signature, Transaction as EthTransaction};
use serde::{Deserialize, Serialize};
use starknet::core::types::{
    BlockId as StarknetBlockId, BlockTag, FieldElement, InvokeTransaction, MaybePendingBlockWithTxHashes,
    MaybePendingBlockWithTxs, Transaction,
};
use starknet::providers::Provider;
use thiserror::Error;

use super::client::errors::EthApiError;
use crate::client::client_api::KakarotClient;
use crate::client::constants::{
    self, CHAIN_ID, DIFFICULTY, GAS_LIMIT, GAS_USED, MIX_HASH, NONCE, SIZE, TOTAL_DIFFICULTY,
};
use crate::client::helpers::{
    decode_signature_from_tx_calldata, starknet_address_to_ethereum_address, vec_felt_to_bytes, DataDecodingError,
};
use crate::models::convertible::ConvertibleStarknetTransaction;

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

#[derive(Debug, Error)]
pub enum ConversionError {
    #[error("transaction conversion error: {0}")]
    TransactionConversionError(String),
    #[error(transparent)]
    DataDecodingError(#[from] DataDecodingError),
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
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, EthApiError> {
        // TODO: Fetch real data
        let gas_limit = *GAS_LIMIT;

        // TODO: Fetch real data
        let gas_used = *GAS_USED;

        // TODO: Fetch real data
        let difficulty = *DIFFICULTY;

        // TODO: Fetch real data
        let nonce: Option<H64> = Some(H64::zero());

        // TODO: Fetch real data
        let size: Option<U256> = *SIZE;

        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data = Bytes::from(b"0x00");

        // TODO: Fetch real data
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = *MIX_HASH;

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
        let block = Block {
            header,
            total_difficulty: *TOTAL_DIFFICULTY,
            uncles: vec![],
            transactions,
            size,
            withdrawals: Some(vec![]),
        };
        Ok(block.into())
    }
}

#[async_trait]
impl ConvertibleStarknetBlock for BlockWithTxs {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, EthApiError> {
        // TODO: Fetch real data
        let gas_limit = *GAS_LIMIT;

        // TODO: Fetch real data
        let gas_used = *GAS_USED;

        // TODO: Fetch real data
        let difficulty = *DIFFICULTY;

        // TODO: Fetch real data
        let nonce: Option<H64> = *NONCE;

        // TODO: Fetch real data
        let size: Option<U256> = *SIZE;

        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data: Bytes = Bytes::from(b"0x00");

        // TODO: Fetch real data
        let base_fee_per_gas = client.base_fee_per_gas();
        // TODO: Fetch real data
        let mix_hash = *MIX_HASH;

        let parent_hash = H256::from_slice(&self.parent_hash().to_bytes_be());

        let sequencer = starknet_address_to_ethereum_address(&self.sequencer_address());

        let timestamp = U256::from(self.timestamp());

        let hash = self.block_hash().as_ref().map(|hash| H256::from_slice(&hash.to_bytes_be()));
        let number = self.block_number().map(U256::from);

        let transactions = client.filter_starknet_into_eth_txs(self.transactions().into(), hash, number).await?;
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
        let block = Block {
            header,
            total_difficulty: *TOTAL_DIFFICULTY,
            uncles: vec![],
            transactions,
            size,
            withdrawals: Some(vec![]),
        };
        Ok(block.into())
    }
}

pub struct Felt252Wrapper(FieldElement);

impl From<FieldElement> for Felt252Wrapper {
    fn from(felt: FieldElement) -> Self {
        Self(felt)
    }
}

impl From<Felt252Wrapper> for FieldElement {
    fn from(felt: Felt252Wrapper) -> Self {
        felt.0
    }
}

impl From<Felt252Wrapper> for H256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        H256::from_slice(&felt.to_bytes_be())
    }
}

impl From<Felt252Wrapper> for U256 {
    fn from(felt: Felt252Wrapper) -> Self {
        let felt: FieldElement = felt.into();
        U256::from_be_bytes(felt.to_bytes_be())
    }
}

pub struct StarknetTransactions(Vec<Transaction>);

impl From<Vec<Transaction>> for StarknetTransactions {
    fn from(txs: Vec<Transaction>) -> Self {
        Self(txs)
    }
}

impl From<StarknetTransactions> for Vec<Transaction> {
    fn from(txs: StarknetTransactions) -> Self {
        txs.0
    }
}

pub struct StarknetTransaction(Transaction);

impl From<Transaction> for StarknetTransaction {
    fn from(tx: Transaction) -> Self {
        Self(tx)
    }
}

impl From<StarknetTransaction> for Transaction {
    fn from(tx: StarknetTransaction) -> Self {
        tx.0
    }
}

macro_rules! get_invoke_transaction_field {
    (($field_v0:ident, $field_v1:ident), $type:ty) => {
        pub fn $field_v1(&self) -> Result<$type, ConversionError> {
            match &self.0 {
                Transaction::Invoke(tx) => match tx {
                    InvokeTransaction::V0(tx) => Ok(tx.$field_v0.clone().into()),
                    InvokeTransaction::V1(tx) => Ok(tx.$field_v1.clone().into()),
                },
                _ => Err(ConversionError::TransactionConversionError(
                    constants::error_messages::INVALID_TRANSACTION_TYPE.to_string(),
                )),
            }
        }
    };
}

impl StarknetTransaction {
    get_invoke_transaction_field!((transaction_hash, transaction_hash), Felt252Wrapper);
    get_invoke_transaction_field!((nonce, nonce), Felt252Wrapper);
    get_invoke_transaction_field!((calldata, calldata), Vec<FieldElement>);
    get_invoke_transaction_field!((contract_address, sender_address), Felt252Wrapper);
}

#[async_trait]
impl ConvertibleStarknetTransaction for StarknetTransaction {
    async fn to_eth_transaction(
        &self,
        client: &dyn KakarotClient,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError> {
        let starknet_block_latest = StarknetBlockId::Tag(BlockTag::Latest);
        let sender_address: FieldElement = self.sender_address()?.into();

        let class_hash = client.inner().get_class_hash_at(starknet_block_latest, sender_address).await?;

        if class_hash != client.proxy_account_class_hash() {
            return Err(EthApiError::OtherError(anyhow::anyhow!("Kakarot Filter: Tx is not part of Kakarot")));
        }

        let hash: H256 = self.transaction_hash()?.into();

        let nonce: U256 = self.nonce()?.into();

        let from = client.get_evm_address(&sender_address, &starknet_block_latest).await?;

        let max_priority_fee_per_gas = Some(client.max_priority_fee_per_gas());

        let calldata = self.calldata().unwrap_or_default();
        let input = vec_felt_to_bytes(calldata.clone());

        // TODO: wrap to abstract the following lines?
        // Extracting the signature
        let signature = decode_signature_from_tx_calldata(&calldata)?;
        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
        let signature = Some(Signature { r: signature.r, s: signature.s, v: U256::from_limbs_slice(&[v]) });

        Ok(EthTransaction {
            hash,
            nonce,
            block_hash,
            block_number,
            transaction_index,
            from,
            to: None,               // TODO fetch the to
            value: U256::from(100), // TODO fetch the value
            gas_price: None,        // TODO fetch the gas price
            gas: U256::from(100),   // TODO fetch the gas amount
            max_fee_per_gas: None,  // TODO fetch the max_fee_per_gas
            max_priority_fee_per_gas,
            input,
            signature,
            chain_id: Some(CHAIN_ID.into()),
            access_list: None,      // TODO fetch the access list
            transaction_type: None, // TODO fetch the transaction type
        })
    }
}
