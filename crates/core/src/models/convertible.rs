use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Log, RichBlock, Transaction as EthTransaction};

use crate::client::client_api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::errors::EthApiError;

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block(&self, client: &dyn KakarotEthApi) -> Result<RichBlock, EthApiError>;
}

pub trait ConvertibleStarknetEvent {
    fn to_eth_log(
        self,
        client: &dyn KakarotStarknetApi,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError>;
}

#[async_trait]
pub trait ConvertibleStarknetTransaction {
    async fn to_eth_transaction(
        &self,
        client: &dyn KakarotEthApi,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError>;
}
