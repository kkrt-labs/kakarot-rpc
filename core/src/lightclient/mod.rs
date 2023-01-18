use eyre::Result;
use jsonrpsee::types::error::CallError;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcClient};
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum LightClientError {
    #[error(transparent)]
    RequestError(#[from] anyhow::Error),
}

pub struct StarknetClient {
    client: JsonRpcClient<HttpTransport>,
}

impl From<LightClientError> for jsonrpsee::core::Error {
    fn from(err: LightClientError) -> Self {
        match err {
            LightClientError::RequestError(e) => jsonrpsee::core::Error::Call(CallError::Failed(e)),
        }
    }
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
    /// `Err(LightClientError)` if the operation failed.
    pub async fn block_number(&self) -> Result<u64, LightClientError> {
        let block_number = self.client.block_number().await.map_err(|e| {
            LightClientError::RequestError(anyhow::anyhow!(
                "Failed to get block number from Starknet RPC: {}",
                e
            ))
        })?;
        Ok(block_number)
    }
}
