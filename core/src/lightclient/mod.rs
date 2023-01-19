use eyre::Result;
use jsonrpsee::types::error::CallError;
use reth_rpc_types::RichBlock;
use starknet::{
    core::types::FieldElement,
    macros::selector,
    providers::jsonrpc::{
        models::{BlockId as StarknetBlockId, FunctionCall},
        HttpTransport, JsonRpcClient, JsonRpcClientError,
    },
};
use thiserror::Error;
use url::Url;
extern crate hex;
use reth_primitives::{Address, Bytes};

use crate::helpers::{starknet_block_to_eth_block, MaybePendingStarknetBlock};

use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;

#[derive(Error, Debug)]
pub enum LightClientError {
    #[error(transparent)]
    RequestError(#[from] JsonRpcClientError<reqwest::Error>),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[automock]
#[async_trait]
pub trait StarknetClient: Send + Sync {
    async fn block_number(&self) -> Result<u64, LightClientError>;
    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, LightClientError>;
    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError>;
}
pub struct StarknetClientImpl {
    client: JsonRpcClient<HttpTransport>,
    // kakarot_contract_address: FieldElement,
    kakarot_account_registry: FieldElement,
}

impl From<LightClientError> for jsonrpsee::core::Error {
    fn from(err: LightClientError) -> Self {
        match err {
            LightClientError::RequestError(e) => jsonrpsee::core::Error::Call(CallError::Failed(
                anyhow::anyhow!("Kakarot Core: Light Client Request Error: {}", e),
            )),
            LightClientError::OtherError(e) => jsonrpsee::core::Error::Call(CallError::Failed(e)),
        }
    }
}

impl StarknetClientImpl {
    pub fn new(starknet_rpc: &str) -> Result<Self> {
        let url = Url::parse(starknet_rpc)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
            // kakarot_contract_address: FieldElement::from_hex_be(
            //     "0x031ddf73d0285cc2f08bd4a2c93229f595f2f6e64b25846fc0957a2faa7ef7bb",
            // )
            // .unwrap(),
            kakarot_account_registry: FieldElement::from_hex_be(
                "0x052a419fd88f53f9a29d22c3d8db24dd9a9a01a41a483ac660d88622f83c40db",
            )
            .unwrap(),
        })
    }
}

#[async_trait]
impl StarknetClient for StarknetClientImpl {
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
    async fn block_number(&self) -> Result<u64, LightClientError> {
        let block_number = self.client.block_number().await?;
        Ok(block_number)
    }

    /// Get the block given a block id.
    /// The block.
    /// # Arguments
    /// * `block_id(StarknetBlockId)` - The block id.
    /// * `hydrated_tx(bool)` - Whether to hydrate the transactions.
    /// # Returns
    /// `Ok(RichBlock)` if the operation was successful.
    /// `Err(LightClientError)` if the operation failed.
    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, LightClientError> {
        let starknet_block = if hydrated_tx {
            MaybePendingStarknetBlock::BlockWithTxs(
                self.client.get_block_with_txs(&block_id).await?,
            )
        } else {
            MaybePendingStarknetBlock::BlockWithTxHashes(
                self.client.get_block_with_tx_hashes(&block_id).await?,
            )
        };
        let block = starknet_block_to_eth_block(starknet_block);
        Ok(block)
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
    /// `Ok(Bytes)` if the operation was successful.
    /// `Err(LightClientError)` if the operation failed.
    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError> {
        // Convert the Ethereum address to a hex-encoded string
        let address_hex = hex::encode(ethereum_address);
        // Convert the hex-encoded string to a FieldElement
        let ethereum_address_field_element = FieldElement::from_hex_be(&address_hex).unwrap();

        // Prepare the calldata for the get_starknet_contract_address function call
        let tx_calldata_vec = vec![ethereum_address_field_element];
        let request = FunctionCall {
            contract_address: self.kakarot_account_registry,
            entry_point_selector: selector!("get_starknet_contract_address"),
            calldata: tx_calldata_vec,
        };
        // Make the function call to get the Starknet contract address
        let starknet_contract_address = self.client.call(request, &starknet_block_id).await?;
        // Concatenate the result of the function call
        let concatenated_result = starknet_contract_address
            .into_iter()
            .fold(FieldElement::ZERO, |acc, x| acc + x);

        // Prepare the calldata for the bytecode function call
        let tx_calldata_vec2 = vec![];
        let request = FunctionCall {
            contract_address: concatenated_result,
            entry_point_selector: selector!("bytecode"),
            calldata: tx_calldata_vec2,
        };
        // Make the function call to get the contract bytecode
        let contract_bytecode = self.client.call(request, &starknet_block_id).await?;
        // Convert the result of the function call to a vector of bytes
        let contract_bytecode_in_u8: Vec<u8> = contract_bytecode
            .into_iter()
            .flat_map(|x| x.to_bytes_be())
            .collect();
        let bytes_result = Bytes::from(contract_bytecode_in_u8);

        // Return the bytecode as a Result<Bytes>
        Ok(bytes_result)
    }
}
