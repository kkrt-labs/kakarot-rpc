use async_trait::async_trait;
use reth_rpc_types::{Log, RichBlock};

use crate::client::client_api::{KakarotClient, KakarotClientError};

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block(&self, client: &dyn KakarotClient) -> Result<RichBlock, KakarotClientError>;
}

#[async_trait]
pub trait ConvertibleStarknetEvent {
    async fn to_eth_log(&self, client: &dyn KakarotClient) -> Result<Log, KakarotClientError>;
}
