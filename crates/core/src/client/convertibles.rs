use async_trait::async_trait;
use reth_primitives::{H256, U256};
use reth_rpc_types::{RichBlock, Transaction as EthTransaction};

use super::client_api::{KakarotClient, KakarotClientError};

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, KakarotClientError>;
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
