use std::sync::Arc;

use async_trait::async_trait;
use eyre::Result;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, H256, U128, U256, U64};
use reth_rpc_types::{
    BlockTransactions, CallRequest, FeeHistory, Filter, FilterChanges, Index, RichBlock, SyncStatus,
    Transaction as EtherTransaction, TransactionReceipt,
};
use starknet::core::types::{
    BlockId as StarknetBlockId, BroadcastedInvokeTransaction, EmittedEvent, EventFilterWithPage, FieldElement,
};
use starknet::providers::sequencer::models::TransactionSimulationInfo;
use starknet::providers::Provider;

use super::errors::EthApiError;
use crate::models::balance::TokenBalances;
use crate::models::transaction::block_tx::StarknetTransactions;

#[async_trait]
pub trait KakarotEthApi<P: Provider + Send + Sync>: KakarotStarknetApi<P> + Send + Sync {
    async fn block_number(&self) -> Result<U64, EthApiError<P::Error>>;

    async fn transaction_by_hash(&self, hash: H256) -> Result<Option<EtherTransaction>, EthApiError<P::Error>>;

    async fn get_code(&self, ethereum_address: Address, block_id: BlockId) -> Result<Bytes, EthApiError<P::Error>>;

    async fn get_logs(&self, filter: Filter) -> Result<FilterChanges, EthApiError<P::Error>>;

    async fn call(
        &self,
        origin: Address,
        to: Address,
        calldata: Bytes,
        block_id: BlockId,
    ) -> Result<Bytes, EthApiError<P::Error>>;

    async fn transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, EthApiError<P::Error>>;

    async fn syncing(&self) -> Result<SyncStatus, EthApiError<P::Error>>;

    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<U64, EthApiError<P::Error>>;

    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64, EthApiError<P::Error>>;

    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>, EthApiError<P::Error>>;

    async fn nonce(&self, ethereum_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>>;

    async fn balance(&self, ethereum_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>>;

    async fn storage_at(
        &self,
        ethereum_address: Address,
        index: U256,
        block_id: BlockId,
    ) -> Result<U256, EthApiError<P::Error>>;

    async fn token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, EthApiError<P::Error>>;

    async fn send_transaction(&self, bytes: Bytes) -> Result<H256, EthApiError<P::Error>>;

    async fn get_transaction_count_by_block(&self, block_id: BlockId) -> Result<U64, EthApiError<P::Error>>;

    fn base_fee_per_gas(&self) -> U256;

    fn max_priority_fee_per_gas(&self) -> U128;

    async fn fee_history(
        &self,
        block_count: U256,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory, EthApiError<P::Error>>;

    async fn estimate_gas(&self, request: CallRequest, block_id: BlockId) -> Result<U256, EthApiError<P::Error>>;

    async fn gas_price(&self) -> Result<U256, EthApiError<P::Error>>;
}

#[async_trait]
pub trait KakarotStarknetApi<P: Provider + Send + Sync>: Send + Sync {
    fn kakarot_address(&self) -> FieldElement;

    fn externally_owned_account_class_hash(&self) -> FieldElement;

    fn contract_account_class_hash(&self) -> FieldElement;

    fn proxy_account_class_hash(&self) -> FieldElement;

    fn starknet_provider(&self) -> Arc<P>;

    async fn map_block_id_to_block_number(&self, block_id: &StarknetBlockId) -> Result<u64, EthApiError<P::Error>>;

    async fn submit_starknet_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
    ) -> Result<H256, EthApiError<P::Error>>;

    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>>;

    async fn get_evm_address(
        &self,
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, EthApiError<P::Error>>;

    async fn filter_starknet_into_eth_txs(
        &self,
        initial_transactions: StarknetTransactions,
        blockhash_opt: Option<H256>,
        blocknum_opt: Option<U256>,
    ) -> BlockTransactions;

    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, EthApiError<P::Error>>;

    async fn simulate_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
        block_number: u64,
        skip_validate: bool,
    ) -> Result<TransactionSimulationInfo, EthApiError<P::Error>>;

    async fn filter_events(&self, request: EventFilterWithPage) -> Result<Vec<EmittedEvent>, EthApiError<P::Error>>;

    async fn wait_for_confirmation_on_l2(&self, transaction_hash: FieldElement) -> Result<(), EthApiError<P::Error>>;
}
