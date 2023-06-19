use reth_rpc_types::RichBlock;

use super::client_api::{KakarotClient, KakarotClientError};

pub trait ConvertibleStarknetBlock {
    fn to_eth_block(&self, client: Box<dyn KakarotClient>)
        -> Result<RichBlock, KakarotClientError>;
}
