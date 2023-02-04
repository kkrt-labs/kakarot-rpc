use eyre::Result;
use jsonrpsee::types::error::CallError;

use reth_primitives::{Address, Bytes, U256};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{BlockId as StarknetBlockId, FunctionCall},
        HttpTransport, JsonRpcClient, JsonRpcClientError,
    },
};
use thiserror::Error;
use url::Url;
extern crate hex;

use crate::helpers::{
    decode_execute_at_address_return, starknet_block_to_eth_block, FeltOrFeltArray,
    MaybePendingStarknetBlock,
};

use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;
pub mod constants;
use constants::{
    selectors::{BYTECODE, GET_STARKNET_CONTRACT_ADDRESS, STORAGE},
    ACCOUNT_REGISTRY_ADDRESS, KAKAROT_MAIN_CONTRACT_ADDRESS,
};
pub mod types;
use types::RichBlock;

use self::constants::selectors::EXECUTE_AT_ADDRESS;

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
    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError>;
    async fn storage_at(
        &self,
        ethereum_address: Address,
        index: U256,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError>;
}
pub struct StarknetClientImpl {
    client: JsonRpcClient<HttpTransport>,
    kakarot_account_registry: FieldElement,
    kakarot_main_contract: FieldElement,
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
        let kakarot_account_registry = FieldElement::from_hex_be(ACCOUNT_REGISTRY_ADDRESS)?;
        let kakarot_main_contract = FieldElement::from_hex_be(KAKAROT_MAIN_CONTRACT_ADDRESS)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
            kakarot_account_registry,
            kakarot_main_contract,
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
        // let hydrated_tx = false;
        let starknet_block = if hydrated_tx {
            MaybePendingStarknetBlock::BlockWithTxs(
                self.client.get_block_with_txs(&block_id).await?,
            )
        } else {
            MaybePendingStarknetBlock::BlockWithTxHashes(
                self.client.get_block_with_tx_hashes(&block_id).await?,
            )
        };
        // fetch gas limit, public key, and nonce from starknet rpc

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
        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            LightClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;

        // Prepare the calldata for the get_starknet_contract_address function call
        let tx_calldata_vec = vec![ethereum_address_felt];
        let request = FunctionCall {
            contract_address: self.kakarot_account_registry,
            entry_point_selector: GET_STARKNET_CONTRACT_ADDRESS,
            calldata: tx_calldata_vec,
        };
        // Make the function call to get the Starknet contract address
        let starknet_contract_address = self.client.call(request, &starknet_block_id).await?;
        // Concatenate the result of the function call
        let concatenated_result = starknet_contract_address
            .into_iter()
            .fold(FieldElement::ZERO, |acc, x| acc + x);

        // Prepare the calldata for the bytecode function call
        let request = FunctionCall {
            contract_address: concatenated_result,
            entry_point_selector: BYTECODE,
            calldata: vec![],
        };
        // Make the function call to get the contract bytecode
        let contract_bytecode = self.client.call(request, &starknet_block_id).await?;
        // Convert the result of the function call to a vector of bytes
        let contract_bytecode_in_u8: Vec<u8> = contract_bytecode
            .into_iter()
            .flat_map(|x| x.to_bytes_be())
            .collect();
        let bytes_result = Bytes::from(contract_bytecode_in_u8);
        Ok(bytes_result)
    }
    // Return the bytecode as a Result<Bytes>
    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError> {
        let address_hex = hex::encode(ethereum_address);

        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            LightClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;

        let mut calldata_vec = calldata
            .clone()
            .into_iter()
            .map(FieldElement::from)
            .collect::<Vec<FieldElement>>();

        let mut call_parameters = vec![
            ethereum_address_felt,
            FieldElement::ZERO,
            FieldElement::MAX,
            calldata.len().into(),
        ];

        call_parameters.append(&mut calldata_vec);

        let request = FunctionCall {
            contract_address: self.kakarot_main_contract,
            entry_point_selector: EXECUTE_AT_ADDRESS,
            calldata: call_parameters,
        };

        let call_result: Vec<FieldElement> = self.client.call(request, &starknet_block_id).await?;

        // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
        // fail is Kakarot API changes)
        // Declare Vec of Result
        // TODO: Change to decode based on ABI or use starknet-rs future feature to decode return
        // params
        let segmented_result = decode_execute_at_address_return(call_result)?;

        // Convert the result of the function call to a vector of bytes
        let return_data = segmented_result.last().ok_or_else(|| {
            LightClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode last argument of Kakarot call",
            ))
        })?;
        if let FeltOrFeltArray::FeltArray(felt_array) = return_data {
            let result: Vec<u8> = felt_array
                .iter()
                .map(|x| x.to_string())
                .filter_map(|s| s.parse().ok())
                .collect();
            let bytes_result = Bytes::from(result);
            return Ok(bytes_result);
        }
        Err(LightClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode the return data of Kakarot call"
        )))
    }
    async fn storage_at(
        &self,
        ethereum_address: Address,
        index: U256,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, LightClientError> {
        // Convert the Ethereum address to a hex-encoded string
        let address_hex = hex::encode(ethereum_address);
        // Convert the hex-encoded string to a FieldElement
        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            LightClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;
        // Prepare the calldata for the get_starknet_contract_address function call
        let tx_calldata_vec = vec![ethereum_address_felt];
        let request = FunctionCall {
            contract_address: self.kakarot_account_registry,
            entry_point_selector: GET_STARKNET_CONTRACT_ADDRESS,
            calldata: tx_calldata_vec,
        };
        // Make the function call to get the Starknet contract address
        let starknet_contract_address = self.client.call(request, &starknet_block_id).await?;
        // Concatenate the result of the function call
        let concatenated_result = starknet_contract_address
            .into_iter()
            .fold(FieldElement::ZERO, |acc, x| acc + x);

        // Prepare the calldata for the storage function call
        // FieldElement::from(index);
        let bytes = index.to_be_bytes();

        let f = FieldElement::from_bytes_be(&bytes).unwrap();
        let request = FunctionCall {
            contract_address: concatenated_result,
            entry_point_selector: STORAGE,
            calldata: vec![f],
        };

        // Make function call to get the contract storage
        let contract_storage = self.client.call(request, &starknet_block_id).await?;

        // Convert the result of the function call to bytes
        let contract_storage_in_u8: Vec<u8> = contract_storage
            .into_iter()
            .flat_map(|x| x.to_bytes_be())
            .collect();
        let bytes_result = Bytes::from(contract_storage_in_u8);

        Ok(bytes_result)
    }
}
