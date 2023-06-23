use async_trait::async_trait;
use eyre::Result;
// TODO: all reth_primitives::rpc types should be replaced when native reth Log is implemented
// https://github.com/paradigmxyz/reth/issues/1396#issuecomment-1440890689
use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, H256, U128, U256, U64};
use reth_rpc_types::{
    BlockTransactions, CallRequest, FeeHistory, Index, RichBlock, SyncStatus, Transaction as EtherTransaction,
    TransactionReceipt,
};
use starknet::core::types::{BlockId as StarknetBlockId, BroadcastedInvokeTransactionV1, FieldElement};
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

use super::errors::EthApiError;
use crate::models::balance::TokenBalances;
use crate::models::transaction::StarknetTransactions;

#[async_trait]
pub trait KakarotClient: Send + Sync {
    fn kakarot_address(&self) -> FieldElement;
    fn proxy_account_class_hash(&self) -> FieldElement;
    fn inner(&self) -> &JsonRpcClient<HttpTransport>;

    async fn block_number(&self) -> Result<U64, EthApiError>;

    async fn transaction_by_hash(&self, hash: H256) -> Result<EtherTransaction, EthApiError>;

    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, EthApiError>;

    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, EthApiError>;

    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, EthApiError>;

    async fn transaction_by_block_id_and_index(
        &self,
        block_id: StarknetBlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, EthApiError>;

    async fn syncing(&self) -> Result<SyncStatus, EthApiError>;

    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<U64, EthApiError>;

    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64, EthApiError>;

    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, EthApiError>;

    async fn submit_starknet_transaction(&self, request: BroadcastedInvokeTransactionV1) -> Result<H256, EthApiError>;

    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>, EthApiError>;

    async fn get_evm_address(
        &self,
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, EthApiError>;

    async fn nonce(&self, ethereum_address: Address, starknet_block_id: StarknetBlockId) -> Result<U256, EthApiError>;

    async fn balance(&self, ethereum_address: Address, starknet_block_id: StarknetBlockId)
    -> Result<U256, EthApiError>;

    async fn token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, EthApiError>;

    async fn filter_starknet_into_eth_txs(
        &self,
        initial_transactions: StarknetTransactions,
        blockhash_opt: Option<H256>,
        blocknum_opt: Option<U256>,
    ) -> Result<BlockTransactions, EthApiError>;

    async fn send_transaction(&self, bytes: Bytes) -> Result<H256, EthApiError>;

    async fn get_transaction_count_by_block(&self, starknet_block_id: StarknetBlockId) -> Result<U64, EthApiError>;

    fn base_fee_per_gas(&self) -> U256;

    fn max_priority_fee_per_gas(&self) -> U128;

    async fn fee_history(
        &self,
        _block_count: U256,
        _newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory, EthApiError>;

    async fn estimate_gas(&self, call_request: CallRequest, block_number: Option<BlockId>)
    -> Result<U256, EthApiError>;
}
