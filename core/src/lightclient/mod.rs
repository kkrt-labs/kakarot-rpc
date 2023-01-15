use eyre::Result;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use url::Url;

pub struct StarknetClient {
    client: JsonRpcClient<HttpTransport>,
}

impl StarknetClient {
    pub fn new(starknet_rpc: &str) -> Result<Self> {
        let url = Url::parse(starknet_rpc)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
        })
    }

    /// Get the number of transactions in a block given a block id.
    /// The number of transactions in a block.
    ///
    /// # Arguments
    ///
    ///
    ///
    /// # Returns
    ///
    ///  * `block_number(u64)` - The block number.
    ///
    /// `Ok(ContractClass)` if the operation was successful.
    /// `Err(eyre::Report)` if the operation failed.
    pub async fn block_number(&self) -> Result<u64> {
        self.client.block_number().await.map_err(|e| eyre::eyre!(e))
    }
}
