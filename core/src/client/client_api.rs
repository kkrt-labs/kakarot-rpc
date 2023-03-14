use eyre::Result;

// TODO: all reth_primitives::rpc types should be replaced when native reth Log is implemented
// https://github.com/paradigmxyz/reth/issues/1396#issuecomment-1440890689
use reth_primitives::{Address, BlockNumberOrTag, Bytes, H256, U256, U64};
use reth_rpc_types::{
    BlockTransactions, RichBlock, SyncStatus, Transaction as EtherTransaction, TransactionReceipt,
};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId as StarknetBlockId, Transaction as StarknetTransaction},
        JsonRpcClientError,
    },
};

use thiserror::Error;
extern crate hex;
use crate::helpers::MaybePendingStarknetBlock;

use crate::client::types::TokenBalances;
use async_trait::async_trait;
use reth_rpc_types::Index;

#[derive(Debug, Error)]
pub enum KakarotClientError {
    #[error(transparent)]
    RequestError(#[from] JsonRpcClientError<reqwest::Error>),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[async_trait]
pub trait KakarotClient: Send + Sync {
    async fn block_number(&self) -> Result<U64, KakarotClientError>;

    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, KakarotClientError>;

    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, KakarotClientError>;

    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, KakarotClientError>;
    async fn transaction_by_block_id_and_index(
        &self,
        block_id: StarknetBlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, KakarotClientError>;
    async fn syncing(&self) -> Result<SyncStatus, KakarotClientError>;
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> Result<U64, KakarotClientError>;
    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64, KakarotClientError>;
    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, KakarotClientError>;
    async fn submit_starknet_transaction(
        &self,
        max_fee: FieldElement,
        signature: Vec<FieldElement>,
        nonce: FieldElement,
        sender_address: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<H256, KakarotClientError>;
    async fn transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, KakarotClientError>;
    async fn get_evm_address(
        &self,
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, KakarotClientError>;
    async fn balance(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<U256, KakarotClientError>;
    async fn token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, KakarotClientError>;
    async fn starknet_tx_into_kakarot_tx(
        &self,
        tx: StarknetTransaction,
        block_hash: Option<H256>,
        block_number: Option<U256>,
    ) -> Result<EtherTransaction, KakarotClientError>;
    async fn starknet_block_to_eth_block(
        &self,
        block: MaybePendingStarknetBlock,
    ) -> Result<RichBlock, KakarotClientError>;
    async fn filter_starknet_into_eth_txs(
        &self,
        initial_transactions: Vec<StarknetTransaction>,
        blockhash_opt: Option<H256>,
        blocknum_opt: Option<U256>,
    ) -> Result<BlockTransactions, KakarotClientError>;
    async fn get_transaction_count_by_block(
        &self,
        starknet_block_id: StarknetBlockId,
    ) -> Result<U64, KakarotClientError>;
}
