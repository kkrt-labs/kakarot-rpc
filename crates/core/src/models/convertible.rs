use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Log, RichBlock, Transaction as EthTransaction};

use crate::client::client_api::{KakarotClient, KakarotClientError};

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, KakarotClientError>;
}

#[async_trait]
pub trait ConvertibleStarknetEvent {
    async fn to_eth_log(
        &self,
        client: &dyn KakarotClient,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, KakarotClientError>;
}

#[async_trait]
pub trait ConvertibleStarknetTransaction {
    async fn to_eth_transaction(
        &self,
        client: &dyn KakarotClient,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, KakarotClientError>;
}
