use eyre::Result;
use jsonrpsee::types::error::CallError;

use reth_primitives::{
    rpc::{BlockNumber, Log, H256},
    Address, Bloom, Bytes, H160, H256 as PrimitiveH256, U128, U256, U64,
};
use reth_rpc_types::{SyncInfo, SyncStatus, TransactionReceipt};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{
            BlockId as StarknetBlockId, FunctionCall, InvokeTransaction,
            MaybePendingBlockWithTxHashes, MaybePendingTransactionReceipt, SyncStatusType,
            Transaction as StarknetTransaction, TransactionReceipt as StarknetTransactionReceipt,
            TransactionStatus as StarknetTransactionStatus,
        },
        HttpTransport, JsonRpcClient, JsonRpcClientError,
    },
};

use thiserror::Error;
use url::Url;
extern crate hex;

use crate::helpers::{
    decode_execute_at_address_return, ethers_block_number_to_starknet_block_id, felt_to_u256,
    starknet_address_to_ethereum_address, starknet_block_to_eth_block, starknet_tx_into_eth_tx,
    FeltOrFeltArray, MaybePendingStarknetBlock,
};

use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;
pub mod constants;
use constants::{
    selectors::{BYTECODE, GET_STARKNET_CONTRACT_ADDRESS},
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
    async fn syncing(&self) -> Result<SyncStatus, LightClientError>;
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<U256>, LightClientError>;
    async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, LightClientError>;
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

    /// Get the syncing status of the light client
    /// # Arguments
    /// # Returns
    ///  `Ok(SyncStatus)` if the operation was successful.
    ///  `Err(LightClientError)` if the operation failed.
    async fn syncing(&self) -> Result<SyncStatus, LightClientError> {
        let status = self.client.syncing().await?;

        match status {
            SyncStatusType::NotSyncing => Ok(SyncStatus::None),

            SyncStatusType::Syncing(data) => {
                let starting_block: U256 = U256::from(data.starting_block_num);
                let current_block: U256 = U256::from(data.current_block_num);
                let highest_block: U256 = U256::from(data.highest_block_num);
                let warp_chunks_amount: Option<U256> = None;
                let warp_chunks_processed: Option<U256> = None;

                let status_info = SyncInfo {
                    starting_block,
                    current_block,
                    highest_block,
                    warp_chunks_amount,
                    warp_chunks_processed,
                };

                Ok(SyncStatus::Info(status_info))
            }
        }
    }

    /// Get the number of transactions in a block given a block number.
    /// The number of transactions in a block.
    ///
    /// # Arguments
    ///
    /// * `number(u64)` - The block number.
    ///
    /// # Returns
    ///
    ///  * `transaction_count(U256)` - The number of transactions.
    ///
    /// `Ok(Bytes)` if the operation was successful.
    /// `Err(LightClientError)` if the operation failed.
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<U256>, LightClientError> {
        let starknet_block_id = ethers_block_number_to_starknet_block_id(number)?;
        let starknet_block = self
            .client
            .get_block_with_tx_hashes(&starknet_block_id)
            .await?;
        match starknet_block {
            MaybePendingBlockWithTxHashes::Block(block) => {
                Ok(Some(U256::from(block.transactions.len())))
            }
            MaybePendingBlockWithTxHashes::PendingBlock(_) => Ok(None),
        }
    }

    async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, LightClientError> {
        let mut res_receipt = TransactionReceipt {
            transaction_hash: None,
            transaction_index: None,
            block_hash: None,
            block_number: None,
            from: H160::from(0),
            to: None,
            //TODO: Fetch real data
            cumulative_gas_used: U256::from(1000000),
            gas_used: None,
            contract_address: None,
            // TODO : default log value
            logs: vec![Log::default()],
            // Bloom is a byte array of length 256
            logs_bloom: Bloom::default(),
            //TODO: Fetch real data
            state_root: None,
            status_code: None,
            //TODO: Fetch real data
            effective_gas_price: U128::from(1000000),
            //TODO: Fetch real data
            transaction_type: U256::from(0),
        };
        //TODO: Error when trying to transform 32 bytes hash to FieldElement
        let hash_hex = hex::encode(hash);
        let hash_felt = FieldElement::from_hex_be(&hash_hex).map_err(|e| {
            LightClientError::OtherError(anyhow::anyhow!(
                "Failed to convert Starknet block hash to FieldElement: {}",
                e
            ))
        })?;
        let starknet_tx_receipt = self.client.get_transaction_receipt(hash_felt).await?;

        match starknet_tx_receipt {
            MaybePendingTransactionReceipt::Receipt(receipt) => {
                match receipt {
                    StarknetTransactionReceipt::Invoke(receipt_data) => {
                        res_receipt.transaction_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.transaction_hash.to_bytes_be(),
                        ));
                        res_receipt.gas_used = Some(felt_to_u256(receipt_data.actual_fee));
                        res_receipt.status_code = match receipt_data.status {
                            StarknetTransactionStatus::Pending => Some(U64::from(0)),
                            StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                            StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                            StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                        };
                        res_receipt.block_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.block_hash.to_bytes_be(),
                        ));
                        res_receipt.block_number =
                            Some(felt_to_u256(receipt_data.block_number.into()));
                    }
                    StarknetTransactionReceipt::L1Handler(receipt_data) => {
                        res_receipt.transaction_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.transaction_hash.to_bytes_be(),
                        ));
                        res_receipt.gas_used = Some(felt_to_u256(receipt_data.actual_fee));
                        res_receipt.status_code = match receipt_data.status {
                            StarknetTransactionStatus::Pending => Some(U64::from(0)),
                            StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                            StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                            StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                        };
                        res_receipt.block_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.block_hash.to_bytes_be(),
                        ));
                        res_receipt.block_number =
                            Some(felt_to_u256(receipt_data.block_number.into()));
                    }
                    StarknetTransactionReceipt::Declare(receipt_data) => {
                        res_receipt.transaction_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.transaction_hash.to_bytes_be(),
                        ));
                        res_receipt.gas_used = Some(felt_to_u256(receipt_data.actual_fee));
                        res_receipt.status_code = match receipt_data.status {
                            StarknetTransactionStatus::Pending => Some(U64::from(0)),
                            StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                            StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                            StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                        };
                        res_receipt.block_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.block_hash.to_bytes_be(),
                        ));
                        res_receipt.block_number =
                            Some(felt_to_u256(receipt_data.block_number.into()));
                    }
                    StarknetTransactionReceipt::Deploy(receipt_data) => {
                        res_receipt.transaction_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.transaction_hash.to_bytes_be(),
                        ));
                        res_receipt.gas_used = Some(felt_to_u256(receipt_data.actual_fee));
                        res_receipt.status_code = match receipt_data.status {
                            StarknetTransactionStatus::Pending => Some(U64::from(0)),
                            StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                            StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                            StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                        };
                        res_receipt.block_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.block_hash.to_bytes_be(),
                        ));
                        res_receipt.block_number =
                            Some(felt_to_u256(receipt_data.block_number.into()));
                    }
                    StarknetTransactionReceipt::DeployAccount(receipt_data) => {
                        res_receipt.transaction_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.transaction_hash.to_bytes_be(),
                        ));
                        res_receipt.gas_used = Some(felt_to_u256(receipt_data.actual_fee));
                        res_receipt.status_code = match receipt_data.status {
                            StarknetTransactionStatus::Pending => Some(U64::from(0)),
                            StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                            StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                            StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                        };
                        res_receipt.block_hash = Some(PrimitiveH256::from_slice(
                            &receipt_data.block_hash.to_bytes_be(),
                        ));
                        res_receipt.block_number =
                            Some(felt_to_u256(receipt_data.block_number.into()));
                    }
                };
            }
            // return nothing when PendingReceipt
            MaybePendingTransactionReceipt::PendingReceipt(_) => {
                return Ok(None);
            }
        };

        let starknet_tx = self.client.get_transaction_by_hash(hash_felt).await?;
        match starknet_tx.clone() {
            StarknetTransaction::Invoke(invoke_tx) => {
                match invoke_tx {
                    InvokeTransaction::V0(v0) => {
                        res_receipt.contract_address =
                            Some(starknet_address_to_ethereum_address(v0.contract_address));
                    }
                    InvokeTransaction::V1(_) => res_receipt.contract_address = None,
                };
            }
            StarknetTransaction::L1Handler(l1_handler_tx) => {
                res_receipt.contract_address = Some(starknet_address_to_ethereum_address(
                    l1_handler_tx.contract_address,
                ));
            }
            StarknetTransaction::Declare(_) => {
                res_receipt.contract_address = None;
            }
            StarknetTransaction::Deploy(_) => {
                res_receipt.contract_address = None;
            }
            StarknetTransaction::DeployAccount(_) => {
                res_receipt.contract_address = None;
            }
        };

        let eth_tx = starknet_tx_into_eth_tx(starknet_tx);
        match eth_tx {
            Ok(tx) => {
                res_receipt.from = tx.from;
                res_receipt.to = tx.to;
            }
            _ => {
                return Ok(None);
            }
        };

        Ok(Some(res_receipt))
    }
}

// Receipt information to return
// ✅ transactionHash
// transactionIndex
// ✅ blockHash
// ✅ blockNumber
// ✅ from
// ✅ to : address of receiver, 0 if account deployment
// cumulativeGasUsed : The sum of gas used by this transaction and all preceding transactions in the same block.
// ✅ gasUsed
// ✅ contractAddress
// logs : Array - Array of log objects, which this transaction generated.
// logsBloom
// root : The post-transaction state root. Only specified for transactions included before the Byzantium upgrade.
// ✅ status
// effectiveGasPrice > get from estimateGasPrice
