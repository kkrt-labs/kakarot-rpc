use std::str::FromStr;

use eyre::Result;
use jsonrpsee::types::error::CallError;

use reth_primitives::{
    rpc::{BlockId, BlockNumber, H256},
    Address, Bytes, H256 as PrimitiveH256, U256, U64,
};
use reth_rpc_types::{SyncInfo, SyncStatus, TransactionReceipt};
use starknet::{
    core::types::FieldElement,
    providers::jsonrpc::{
        models::{
            BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction,
            BroadcastedInvokeTransactionV1, DeployAccountTransactionReceipt,
            DeployTransactionReceipt, FunctionCall, InvokeTransaction, InvokeTransactionReceipt,
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
    create_default_transaction_receipt, decode_execute_at_address_return,
    ethers_block_id_to_starknet_block_id, felt_to_u256, hash_to_field_element,
    starknet_address_to_ethereum_address, starknet_block_to_eth_block, starknet_tx_into_eth_tx,
    vec_felt_to_bytes, FeltOrFeltArray, MaybePendingStarknetBlock,
};

use crate::client::types::Transaction as EtherTransaction;
use async_trait::async_trait;
use mockall::predicate::*;
use mockall::*;
use reth_rpc_types::Index;
pub mod constants;
use constants::{selectors::BYTECODE, KAKAROT_MAIN_CONTRACT_ADDRESS};
pub mod types;
use types::{RichBlock, TokenBalance, TokenBalances};

use self::constants::selectors::{COMPUTE_STARKNET_ADDRESS, EXECUTE_AT_ADDRESS, GET_EVM_ADDRESS};

#[derive(Error, Debug)]
pub enum KakarotClientError {
    #[error(transparent)]
    RequestError(#[from] JsonRpcClientError<reqwest::Error>),
    #[error(transparent)]
    OtherError(#[from] anyhow::Error),
}

#[automock]
#[async_trait]
pub trait StarknetClient: Send + Sync {
    async fn block_number(&self) -> Result<U256, KakarotClientError>;

    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, KakarotClientError>;

    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, KakarotClientError>;

    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, KakarotClientError>;
    async fn transaction_by_block_number_and_index(
        &self,
        block_id: StarknetBlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, KakarotClientError>;
    async fn syncing(&self) -> Result<SyncStatus, KakarotClientError>;
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<U256>, KakarotClientError>;
    async fn block_transaction_count_by_hash(
        &self,
        hash: H256,
    ) -> Result<Option<U256>, KakarotClientError>;
    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<FieldElement, KakarotClientError>;
    async fn submit_starknet_transaction(
        &self,
        max_fee: FieldElement,
        signature: Vec<FieldElement>,
        nonce: FieldElement,
        sender_address: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<H256, KakarotClientError>;
    async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, KakarotClientError>;
    async fn get_evm_address(
        &self,
        starknet_address: FieldElement,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Address, KakarotClientError>;
    async fn get_token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, KakarotClientError>;
}
pub struct StarknetClientImpl {
    client: JsonRpcClient<HttpTransport>,
    kakarot_main_contract: FieldElement,
}

impl From<KakarotClientError> for jsonrpsee::core::Error {
    fn from(err: KakarotClientError) -> Self {
        match err {
            KakarotClientError::RequestError(e) => jsonrpsee::core::Error::Call(CallError::Failed(
                anyhow::anyhow!("Kakarot Core: Light Client Request Error: {}", e),
            )),
            KakarotClientError::OtherError(e) => jsonrpsee::core::Error::Call(CallError::Failed(e)),
        }
    }
}

impl StarknetClientImpl {
    pub fn new(starknet_rpc: &str) -> Result<Self> {
        let url = Url::parse(starknet_rpc)?;
        let kakarot_main_contract = FieldElement::from_hex_be(KAKAROT_MAIN_CONTRACT_ADDRESS)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
            kakarot_main_contract,
        })
    }

    /// Get the Ethereum address of a Starknet Kakarot smart-contract by calling get_evm_address on it.
    /// If the contract's get_evm_address errors, returns the Starknet address sliced to 20 bytes to conform with EVM addresses formats.
    ///
    /// ## Arguments
    ///
    /// * `starknet_address` - The Starknet address of the contract.
    /// * `starknet_block_id` - The block id to query the contract at.
    ///
    /// ## Returns
    ///
    /// * `eth_address` - The Ethereum address of the contract.
    pub async fn safe_get_evm_address(
        &self,
        starknet_address: FieldElement,
        starknet_block_id: StarknetBlockId,
    ) -> Address {
        let eth_address = self
            .get_evm_address(starknet_address, starknet_block_id)
            .await
            .unwrap_or_else(|_| starknet_address_to_ethereum_address(starknet_address));
        eth_address
    }
}

#[async_trait]
impl StarknetClient for StarknetClientImpl {
    /// Get the number of transactions in a block given a block id.
    /// The number of transactions in a block.
    ///
    /// ## Arguments
    ///
    ///
    ///
    /// ## Returns
    ///
    ///  * `block_number(u64)` - The block number.
    ///
    /// `Ok(ContractClass)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_number(&self) -> Result<U256, KakarotClientError> {
        let block_number = self.client.block_number().await?;
        Ok(U256::from(block_number))
    }

    /// Get the block given a block id.
    /// The block.
    /// ## Arguments
    /// * `block_id(StarknetBlockId)` - The block id.
    /// * `hydrated_tx(bool)` - Whether to hydrate the transactions.
    /// ## Returns
    /// `Ok(RichBlock)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, KakarotClientError> {
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
    /// ## Arguments
    ///
    ///
    ///
    /// ## Returns
    ///
    ///  * `block_number(u64)` - The block number.
    ///
    /// `Ok(Bytes)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn get_code(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Bytes, KakarotClientError> {
        // Convert the Ethereum address to a hex-encoded string
        let address_hex = hex::encode(ethereum_address);
        // Convert the hex-encoded string to a FieldElement
        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;

        // Prepare the calldata for the get_starknet_contract_address function call
        let tx_calldata_vec = vec![ethereum_address_felt];
        let request = FunctionCall {
            contract_address: self.kakarot_main_contract,
            entry_point_selector: COMPUTE_STARKNET_ADDRESS,
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
    ) -> Result<Bytes, KakarotClientError> {
        let address_hex = hex::encode(ethereum_address);

        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
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
        let return_data = segmented_result.get(6).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
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
        Err(KakarotClientError::OtherError(anyhow::anyhow!(
            "Cannot parse and decode the return data of Kakarot call"
        )))
    }

    /// Get the syncing status of the light client
    /// # Arguments
    /// # Returns
    ///  `Ok(SyncStatus)` if the operation was successful.
    ///  `Err(KakarotClientError)` if the operation failed.
    async fn syncing(&self) -> Result<SyncStatus, KakarotClientError> {
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
    /// `Ok(Option<U256>)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumber,
    ) -> Result<Option<U256>, KakarotClientError> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(BlockId::Number(number))?;
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

    /// Get the number of transactions in a block given a block hash.
    /// The number of transactions in a block.
    /// # Arguments
    /// * `hash(H256)` - The block hash.
    /// # Returns
    ///
    ///  * `transaction_count(U256)` - The number of transactions.
    ///
    /// `Ok(Option<U256>)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_transaction_count_by_hash(
        &self,
        hash: H256,
    ) -> Result<Option<U256>, KakarotClientError> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(BlockId::Hash(hash))?;
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
    async fn transaction_by_block_number_and_index(
        &self,
        block_id: StarknetBlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, KakarotClientError> {
        let usize_index: usize = tx_index.into();
        let index: u64 = usize_index as u64;
        let starknet_tx = self
            .client
            .get_transaction_by_block_id_and_index(&block_id, index)
            .await?;
        let tx_hash = match &starknet_tx {
            StarknetTransaction::Invoke(InvokeTransaction::V0(tx)) => tx.transaction_hash,
            StarknetTransaction::Invoke(InvokeTransaction::V1(tx)) => tx.transaction_hash,
            StarknetTransaction::L1Handler(tx) => tx.transaction_hash,
            StarknetTransaction::Declare(tx) => tx.transaction_hash,
            StarknetTransaction::Deploy(tx) => tx.transaction_hash,
            StarknetTransaction::DeployAccount(tx) => tx.transaction_hash,
        };
        let tx_receipt = self.client.get_transaction_receipt(tx_hash).await?;
        let (blockhash_opt, blocknum_opt) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => (
                Some(PrimitiveH256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::L1Handler(tr)) => (
                Some(PrimitiveH256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Declare(tr)) => (
                Some(PrimitiveH256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Deploy(tr)) => (
                Some(PrimitiveH256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::DeployAccount(
                tr,
            )) => (
                Some(PrimitiveH256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::PendingReceipt(_) => (None, None),
        };
        let eth_tx = starknet_tx_into_eth_tx(starknet_tx, blockhash_opt, blocknum_opt)?;
        Ok(eth_tx)
    }

    /// Returns the Starknet address associated with a given Ethereum address.
    ///
    /// ## Arguments
    /// * `ethereum_address` - The Ethereum address to convert to a Starknet address.
    /// * `starknet_block_id` - The block ID to use for the Starknet contract call.
    ///
    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: StarknetBlockId,
    ) -> Result<FieldElement, KakarotClientError> {
        let address_hex = hex::encode(ethereum_address);

        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;

        let request = FunctionCall {
            contract_address: self.kakarot_main_contract,
            entry_point_selector: COMPUTE_STARKNET_ADDRESS,
            calldata: vec![ethereum_address_felt],
        };

        let starknet_contract_address = self.client.call(request, &starknet_block_id).await?;

        let result = starknet_contract_address.first().ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to get Starknet address from Kakarot"
            ))
        })?;

        println!("Starknet address: {result}");

        Ok(*result)
    }

    async fn submit_starknet_transaction(
        &self,
        max_fee: FieldElement,
        signature: Vec<FieldElement>,
        nonce: FieldElement,
        sender_address: FieldElement,
        calldata: Vec<FieldElement>,
    ) -> Result<H256, KakarotClientError> {
        let transaction_v1 = BroadcastedInvokeTransactionV1 {
            max_fee,
            signature,
            nonce,
            sender_address,
            calldata,
        };

        let transaction_result = self
            .client
            .add_invoke_transaction(&BroadcastedInvokeTransaction::V1(transaction_v1))
            .await?;

        println!(
            "Kakarot Core: Submitted Starknet transaction with hash: {0}",
            transaction_result.transaction_hash,
        );

        Ok(H256::from(
            transaction_result.transaction_hash.to_bytes_be(),
        ))
    }

    /// Returns the receipt of a transaction by transaction hash.
    ///
    /// # Arguments
    ///
    /// * `hash(H256)` - The block hash.
    ///
    /// # Returns
    ///
    ///  * `transaction_receipt(TransactionReceipt)` - The transaction receipt.
    ///
    /// `Ok(Option<TransactionReceipt>)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn get_transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, KakarotClientError> {
        let mut res_receipt = create_default_transaction_receipt();

        //TODO: Error when trying to transform 32 bytes hash to FieldElement
        let hash_felt = hash_to_field_element(PrimitiveH256::from(hash.0))?;
        let starknet_tx_receipt = self.client.get_transaction_receipt(hash_felt).await?;

        match starknet_tx_receipt {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                StarknetTransactionReceipt::Invoke(InvokeTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    ..
                })
                | StarknetTransactionReceipt::Deploy(DeployTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    ..
                })
                | StarknetTransactionReceipt::DeployAccount(DeployAccountTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    ..
                }) => {
                    res_receipt.transaction_hash =
                        Some(PrimitiveH256::from_slice(&transaction_hash.to_bytes_be()));
                    res_receipt.status_code = match status {
                        StarknetTransactionStatus::Pending => Some(U64::from(0)),
                        StarknetTransactionStatus::AcceptedOnL1 => Some(U64::from(1)),
                        StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                        StarknetTransactionStatus::Rejected => Some(U64::from(0)),
                    };
                    res_receipt.block_hash =
                        Some(PrimitiveH256::from_slice(&block_hash.to_bytes_be()));
                    res_receipt.block_number = Some(felt_to_u256(block_number.into()));
                }
                // L1Handler and Declare transactions not supported for now in Kakarot
                StarknetTransactionReceipt::L1Handler(_)
                | StarknetTransactionReceipt::Declare(_) => return Ok(None),
            },
            MaybePendingTransactionReceipt::PendingReceipt(_) => {
                return Ok(None);
            }
        };

        let starknet_tx = self.client.get_transaction_by_hash(hash_felt).await?;
        match starknet_tx.clone() {
            StarknetTransaction::Invoke(invoke_tx) => {
                match invoke_tx {
                    InvokeTransaction::V0(v0) => {
                        let eth_address = self
                            .safe_get_evm_address(
                                v0.contract_address,
                                StarknetBlockId::Tag(BlockTag::Latest),
                            )
                            .await;
                        res_receipt.contract_address = Some(eth_address);
                    }
                    InvokeTransaction::V1(_) => {}
                };
            }
            StarknetTransaction::DeployAccount(_) | StarknetTransaction::Deploy(_) => {}
            _ => return Ok(None),
        };

        let eth_tx = starknet_tx_into_eth_tx(starknet_tx, None, None);
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

    async fn get_evm_address(
        &self,
        starknet_address: FieldElement,
        starknet_block_id: StarknetBlockId,
    ) -> Result<Address, KakarotClientError> {
        let request = FunctionCall {
            contract_address: starknet_address,
            entry_point_selector: GET_EVM_ADDRESS,
            calldata: vec![],
        };

        let evm_address_felt = self.client.call(request, &starknet_block_id).await?;
        let evm_address = evm_address_felt
            .first()
            .ok_or_else(|| {
                KakarotClientError::OtherError(anyhow::anyhow!(
                    "Kakarot Core: Failed to get EVM address from smart contract on Kakarot"
                ))
            })?
            .to_bytes_be();

        // Workaround as .get(12..32) does not dynamically size the slice
        let slice: &[u8] = evm_address.get(12..32).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to cast EVM address from 32 bytes to 20 bytes EVM format"
            ))
        })?;
        let mut tmp_slice = [0u8; 20];
        tmp_slice.copy_from_slice(slice);
        let evm_address_sliced = &tmp_slice;

        Ok(Address::from(evm_address_sliced))
    }

    /// Returns token balances for a specific address given a list of contracts.
    ///
    /// # Arguments
    ///
    /// * `address(Address)` - specific address
    /// * `contract_addresses(Vec<Address>)` - List of contract addresses
    ///
    /// # Returns
    ///
    ///  * `token_balances(TokenBalances)` - Token balances
    ///
    /// `Ok(<TokenBalances>)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn get_token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, KakarotClientError> {
        let mut token_balances = Vec::new();
        let starknet_user_address = self
            .compute_starknet_address(address, StarknetBlockId::Tag(BlockTag::Latest))
            .await?;
        let entrypoint = FieldElement::from_str("balanceOf").map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Failed to convert entrypoint to FieldElement: {}",
                e
            ))
        })?;

        for token_address in contract_addresses.into_iter() {
            let calldata = vec![entrypoint, starknet_user_address];
            let call = self
                .call_view(
                    token_address,
                    vec_felt_to_bytes(calldata),
                    StarknetBlockId::Tag(BlockTag::Latest),
                )
                .await;
            let token_balance = match call {
                Ok(call) => TokenBalance {
                    contract_address: address,
                    token_balance: call.to_string(),
                    error: "".to_string(),
                },
                Err(_) => TokenBalance {
                    contract_address: address,
                    token_balance: "".to_string(),
                    error: "Failed to get token balance".to_string(),
                },
            };
            token_balances.push(token_balance);
        }

        Ok(TokenBalances {
            address,
            token_balances,
        })
    }
}
