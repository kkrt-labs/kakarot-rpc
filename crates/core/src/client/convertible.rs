use async_trait::async_trait;
use reth_rpc_types::RichBlock;

use super::client_api::{KakarotClient, KakarotClientError};

#[async_trait]
pub trait ConvertibleStarknetBlock {
    async fn to_eth_block(
        &self,
        client: &dyn KakarotClient,
    ) -> Result<RichBlock, KakarotClientError>;
}
