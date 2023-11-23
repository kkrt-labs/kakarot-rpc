use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Log, RichBlock, Transaction as EthTransaction, TransactionReceipt};
use starknet::core::types::{BroadcastedInvokeTransaction, EventFilter};
use starknet::providers::Provider;

use crate::client::errors::EthApiError;
use crate::client::KakarotClient;

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block<P: Provider + Send + Sync + 'static>(&self, client: &KakarotClient<P>) -> RichBlock;
}

pub trait ConvertibleStarknetEvent {
    fn to_eth_log<P: Provider + Send + Sync + 'static>(
        self,
        client: &KakarotClient<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError>;
}

pub trait ConvertibleEthEventFilter {
    fn to_starknet_event_filter<P: Provider + Send + Sync + 'static>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<EventFilter, EthApiError>;
}

#[async_trait]
pub trait ConvertibleStarknetTransaction {
    async fn to_eth_transaction<P: Provider + Send + Sync + 'static>(
        &self,
        client: &KakarotClient<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError>;
}

#[async_trait]
pub trait ConvertibleSignedTransaction {
    async fn to_broadcasted_invoke_transaction<P: Provider + Send + Sync + 'static>(
        &self,
        client: &KakarotClient<P>,
    ) -> Result<BroadcastedInvokeTransaction, EthApiError>;
}

#[async_trait]
pub trait ConvertibleStarknetTransactionReceipt {
    async fn to_eth_transaction_receipt<P: Provider + Send + Sync + 'static>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<Option<TransactionReceipt>, EthApiError>;
}
