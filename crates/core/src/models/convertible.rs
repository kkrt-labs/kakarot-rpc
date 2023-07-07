use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Log, RichBlock, Transaction as EthTransaction};
use starknet::providers::Provider;

use crate::client::api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::errors::EthApiError;

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block<P: Provider + Send + Sync>(&self, client: &dyn KakarotEthApi<P>) -> RichBlock;
}

pub trait ConvertibleStarknetEvent {
    fn to_eth_log<P: Provider + Send + Sync>(
        self,
        client: &dyn KakarotStarknetApi<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError<P::Error>>;
}

#[async_trait]
pub trait ConvertibleStarknetTransaction {
    async fn to_eth_transaction<P: Provider + Send + Sync>(
        &self,
        client: &dyn KakarotEthApi<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError<P::Error>>;
}
