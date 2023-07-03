use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{Log, RichBlock, Transaction as EthTransaction};
use starknet::providers::jsonrpc::JsonRpcTransport;

use crate::client::api::{KakarotEthApi, KakarotStarknetApi};
use crate::client::errors::EthApiError;

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block<T: JsonRpcTransport + Send + Sync>(
        &self,
        client: &dyn KakarotEthApi<T>,
    ) -> Result<RichBlock, EthApiError<T::Error>>;
}

pub trait ConvertibleStarknetEvent {
    fn to_eth_log<T: JsonRpcTransport + Send + Sync>(
        self,
        client: &dyn KakarotStarknetApi<T>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError<T::Error>>;
}

#[async_trait]
pub trait ConvertibleStarknetTransaction {
    async fn to_eth_transaction<T: JsonRpcTransport + Send + Sync>(
        &self,
        client: &dyn KakarotEthApi<T>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<EthTransaction, EthApiError<T::Error>>;
}
