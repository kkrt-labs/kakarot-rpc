use std::str::FromStr;

use eyre::Result;
use futures::future::join_all;
use jsonrpsee::types::error::CallError;
use std::convert::From;

// TODO: all reth_primitives::rpc types should be replaced when native reth Log is implemented
// https://github.com/paradigmxyz/reth/issues/1396#issuecomment-1440890689
use reth_primitives::{
    keccak256, Address, BlockId, BlockNumberOrTag, Bloom, Bytes, Bytes as RpcBytes,
    TransactionSigned, H160, H256, H64, U128, U256, U64,
};

use reth_rlp::Decodable;

use reth_rpc_types::{
    Block, BlockTransactions, CallRequest, FeeHistory, Header, Log, Rich, RichBlock, Signature,
    SyncInfo, SyncStatus, Transaction as EtherTransaction, TransactionReceipt,
};
use starknet::{
    core::types::{
        BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction,
        BroadcastedInvokeTransactionV1, DeployAccountTransactionReceipt, DeployTransactionReceipt,
        FieldElement, FunctionCall, InvokeTransaction, InvokeTransactionReceipt,
        MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs, MaybePendingTransactionReceipt,
        SyncStatusType, Transaction as StarknetTransaction,
        TransactionReceipt as StarknetTransactionReceipt,
        TransactionStatus as StarknetTransactionStatus,
    },
    providers::{
        jsonrpc::{HttpTransport, JsonRpcClient},
        Provider,
    },
};

use url::Url;
extern crate hex;
pub mod convertible;
pub mod helpers;

use helpers::{
    create_default_transaction_receipt, decode_eth_call_return, decode_signature_from_tx_calldata,
    ethers_block_id_to_starknet_block_id, felt_to_u256, hash_to_field_element,
    raw_starknet_calldata, starknet_address_to_ethereum_address, vec_felt_to_bytes,
    FeltOrFeltArray, MaybePendingStarknetBlock,
};
use std::collections::BTreeMap;

use crate::client::constants::{selectors::ETH_CALL, CHAIN_ID};
use async_trait::async_trait;
use reth_rpc_types::Index;
pub mod client_api;
pub mod constants;
use constants::selectors::BYTECODE;
pub mod models;
use models::{TokenBalance, TokenBalances};

use self::{
    client_api::{KakarotClient, KakarotClientError},
    constants::{
        gas::{BASE_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS},
        selectors::{BALANCE_OF, COMPUTE_STARKNET_ADDRESS, GET_EVM_ADDRESS},
        STARKNET_NATIVE_TOKEN,
    },
};

pub struct KakarotClientImpl {
    client: JsonRpcClient<HttpTransport>,
    kakarot_address: FieldElement,
    proxy_account_class_hash: FieldElement,
}

impl From<KakarotClientError> for jsonrpsee::core::Error {
    fn from(err: KakarotClientError) -> Self {
        match err {
            KakarotClientError::RequestError(e) => Self::Call(CallError::Failed(anyhow::anyhow!(
                "Kakarot Core: Light Client Request Error: {}",
                e
            ))),
            KakarotClientError::OtherError(e) => Self::Call(CallError::Failed(e)),
        }
    }
}

impl KakarotClientImpl {
    /// Create a new `KakarotClient`.
    ///
    /// # Arguments
    ///
    /// * `starknet_rpc(&str)` - `StarkNet` RPC
    ///
    /// # Errors
    ///
    /// `Err(KakarotClientError)` if the operation failed.
    pub fn new(
        starknet_rpc: &str,
        kakarot_address: FieldElement,
        proxy_account_class_hash: FieldElement,
    ) -> Result<Self> {
        let url = Url::parse(starknet_rpc)?;
        Ok(Self {
            client: JsonRpcClient::new(HttpTransport::new(url)),
            kakarot_address,
            proxy_account_class_hash,
        })
    }

    /// Get the Ethereum address of a Starknet Kakarot smart-contract by calling `get_evm_address`
    /// on it. If the contract's `get_evm_address` errors, returns the Starknet address sliced
    /// to 20 bytes to conform with EVM addresses formats.
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
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Address {
        self.get_evm_address(starknet_address, starknet_block_id)
            .await
            .unwrap_or_else(|_| starknet_address_to_ethereum_address(starknet_address))
    }
}

#[async_trait]
impl KakarotClient for KakarotClientImpl {
    fn kakarot_address(&self) -> FieldElement {
        self.kakarot_address
    }

    fn proxy_account_class_hash(&self) -> FieldElement {
        self.proxy_account_class_hash
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
    /// `Ok(ContractClass)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_number(&self) -> Result<U64, KakarotClientError> {
        let block_number = self.client.block_number().await?;
        Ok(U64::from(block_number))
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
        let starknet_block = if hydrated_tx {
            MaybePendingStarknetBlock::BlockWithTxs(self.client.get_block_with_txs(block_id).await?)
        } else {
            MaybePendingStarknetBlock::BlockWithTxHashes(
                self.client.get_block_with_tx_hashes(block_id).await?,
            )
        };
        let block = self.starknet_block_to_eth_block(starknet_block).await?;
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
            contract_address: self.kakarot_address,
            entry_point_selector: COMPUTE_STARKNET_ADDRESS,
            calldata: tx_calldata_vec,
        };
        // Make the function call to get the Starknet contract address
        let starknet_contract_address = self.client.call(request, starknet_block_id).await?;
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
        let contract_bytecode = self.client.call(request, starknet_block_id).await?;
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
            FieldElement::MAX,
            FieldElement::ZERO,
            FieldElement::ZERO,
            calldata.len().into(),
        ];

        call_parameters.append(&mut calldata_vec);

        let request = FunctionCall {
            contract_address: self.kakarot_address,
            entry_point_selector: ETH_CALL,
            calldata: call_parameters,
        };

        let call_result: Vec<FieldElement> = self.client.call(request, starknet_block_id).await?;

        // Parse and decode Kakarot's call return data (temporary solution and not scalable - will
        // fail is Kakarot API changes)
        // Declare Vec of Result
        // TODO: Change to decode based on ABI or use starknet-rs future feature to decode return
        // params
        let segmented_result = decode_eth_call_return(&call_result)?;

        // Convert the result of the function call to a vector of bytes
        let return_data = segmented_result.get(0).ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Cannot parse and decode last argument of Kakarot call",
            ))
        })?;
        if let FeltOrFeltArray::FeltArray(felt_array) = return_data {
            let result: Vec<u8> = felt_array
                .iter()
                .filter_map(|f| u8::try_from(*f).ok())
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
    ///  * `transaction_count(U64)` - The number of transactions.
    ///
    /// `Ok(U64)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_transaction_count_by_number(
        &self,
        number: BlockNumberOrTag,
    ) -> Result<U64, KakarotClientError> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(BlockId::Number(number))?;
        self.get_transaction_count_by_block(starknet_block_id).await
    }

    /// Get the number of transactions in a block given a block hash.
    /// The number of transactions in a block.
    /// # Arguments
    /// * `hash(H256)` - The block hash.
    /// # Returns
    ///
    ///  * `transaction_count(U64)` - The number of transactions.
    ///
    /// `Ok(U64)` if the operation was successful.
    /// `Err(KakarotClientError)` if the operation failed.
    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64, KakarotClientError> {
        let starknet_block_id = ethers_block_id_to_starknet_block_id(BlockId::Hash(hash.into()))?;
        self.get_transaction_count_by_block(starknet_block_id).await
    }

    async fn get_transaction_count_by_block(
        &self,
        starknet_block_id: StarknetBlockId,
    ) -> Result<U64, KakarotClientError> {
        let starknet_block = self.client.get_block_with_txs(starknet_block_id).await?;

        let block_transactions = match starknet_block {
            MaybePendingBlockWithTxs::PendingBlock(pending_block_with_txs) => {
                self.filter_starknet_into_eth_txs(pending_block_with_txs.transactions, None, None)
                    .await?
            }
            MaybePendingBlockWithTxs::Block(block_with_txs) => {
                let blockhash_opt =
                    Some(H256::from_slice(&(block_with_txs.block_hash).to_bytes_be()));
                let blocknum_opt = Some(U256::from(block_with_txs.block_number));
                self.filter_starknet_into_eth_txs(
                    block_with_txs.transactions,
                    blockhash_opt,
                    blocknum_opt,
                )
                .await?
            }
        };
        let len = match block_transactions {
            BlockTransactions::Full(transactions) => transactions.len(),
            BlockTransactions::Hashes(_) => 0,
            BlockTransactions::Uncle => 0,
        };
        Ok(U64::from(len))
    }

    async fn transaction_by_block_id_and_index(
        &self,
        block_id: StarknetBlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, KakarotClientError> {
        let index: u64 = usize::from(tx_index) as u64;

        let starknet_tx = self
            .client
            .get_transaction_by_block_id_and_index(block_id, index)
            .await?;

        let tx_hash = match &starknet_tx {
            StarknetTransaction::Invoke(InvokeTransaction::V0(tx)) => tx.transaction_hash,
            StarknetTransaction::Invoke(InvokeTransaction::V1(tx)) => tx.transaction_hash,
            StarknetTransaction::L1Handler(_)
            | StarknetTransaction::Declare(_)
            | StarknetTransaction::Deploy(_)
            | StarknetTransaction::DeployAccount(_) => {
                return Err(KakarotClientError::OtherError(anyhow::anyhow!(
                    "Kakarot get_transaction_by_block_id_and_index: L1Handler, Declare, Deploy and DeployAccount transactions unsupported"
                )))
            }
        };

        let tx_receipt = self.client.get_transaction_receipt(tx_hash).await?;
        let (blockhash_opt, blocknum_opt) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => (
                Some(H256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::L1Handler(_)) => {
                (None, None)
            }
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Declare(tr)) => (
                Some(H256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Deploy(tr)) => (
                Some(H256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::DeployAccount(
                tr,
            )) => (
                Some(H256::from_slice(&(tr.block_hash).to_bytes_be())),
                Some(U256::from(tr.block_number)),
            ),
            MaybePendingTransactionReceipt::PendingReceipt(_) => (None, None),
        };
        let eth_tx = self
            .starknet_tx_into_kakarot_tx(starknet_tx, blockhash_opt, blocknum_opt)
            .await?;
        Ok(eth_tx)
    }

    /// Returns the Starknet address associated with a given Ethereum address.
    ///
    /// ## Arguments
    /// * `ethereum_address` - The Ethereum address to convert to a Starknet address.
    /// * `starknet_block_id` - The block ID to use for the Starknet contract call.
    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, KakarotClientError> {
        let address_hex = hex::encode(ethereum_address);

        let ethereum_address_felt = FieldElement::from_hex_be(&address_hex).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to convert Ethereum address to FieldElement: {}",
                e
            ))
        })?;

        let request = FunctionCall {
            contract_address: self.kakarot_address,
            entry_point_selector: COMPUTE_STARKNET_ADDRESS,
            calldata: vec![ethereum_address_felt],
        };

        let starknet_contract_address = self.client.call(request, starknet_block_id).await?;

        let result = starknet_contract_address.first().ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Core: Failed to get Starknet address from Kakarot"
            ))
        })?;

        Ok(*result)
    }

    async fn submit_starknet_transaction(
        &self,
        request: BroadcastedInvokeTransactionV1,
    ) -> Result<H256, KakarotClientError> {
        let transaction_result = self
            .client
            .add_invoke_transaction(&BroadcastedInvokeTransaction::V1(request))
            .await?;

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
    async fn transaction_receipt(
        &self,
        hash: H256,
    ) -> Result<Option<TransactionReceipt>, KakarotClientError> {
        let mut res_receipt = create_default_transaction_receipt();

        //TODO: Error when trying to transform 32 bytes hash to FieldElement
        let hash_felt = hash_to_field_element(H256::from(hash.0))?;
        let starknet_tx_receipt = self.client.get_transaction_receipt(hash_felt).await?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        match starknet_tx_receipt {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                StarknetTransactionReceipt::Invoke(InvokeTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    events,
                    ..
                })
                | StarknetTransactionReceipt::Deploy(DeployTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    events,
                    ..
                })
                | StarknetTransactionReceipt::DeployAccount(DeployAccountTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    events,
                    ..
                }) => {
                    res_receipt.transaction_hash =
                        Some(H256::from(&transaction_hash.to_bytes_be()));
                    res_receipt.status_code = match status {
                        StarknetTransactionStatus::Rejected
                        | StarknetTransactionStatus::Pending => Some(U64::from(0)),
                        StarknetTransactionStatus::AcceptedOnL1
                        | StarknetTransactionStatus::AcceptedOnL2 => Some(U64::from(1)),
                    };
                    res_receipt.block_hash = Some(H256::from(&block_hash.to_bytes_be()));
                    res_receipt.block_number = Some(U256::from(block_number));

                    // Handle events -- Will error if the event is not a Kakarot event
                    let mut tmp_logs = Vec::new();

                    // Cannot use `map` because of the `await` call.
                    for event in events {
                        let contract_address = self
                            .safe_get_evm_address(&event.from_address, &starknet_block_id)
                            .await;

                        // event "keys" in cairo are event "topics" in solidity
                        // they're returned as list where consecutive values are
                        // low, high, low, high, etc. of the Uint256 Cairo representation
                        // of the bytes32 topics. This recomputes the original topic
                        let topics = (0..event.keys.len())
                            .step_by(2)
                            .map(|i| {
                                let next_key =
                                    *event.keys.get(i + 1).unwrap_or(&FieldElement::ZERO);

                                // Can unwrap here as we know 2^128 is a valid FieldElement
                                let two_pow_16: FieldElement = FieldElement::from_hex_be(
                                    "0x100000000000000000000000000000000",
                                )
                                .unwrap();

                                //TODO: May wrap around prime field - Investigate edge cases
                                let felt_shifted_next_key = next_key * two_pow_16;
                                event.keys[i] + felt_shifted_next_key
                            })
                            .map(|topic| H256::from(&topic.to_bytes_be()))
                            .collect::<Vec<_>>();

                        let data = vec_felt_to_bytes(event.data);

                        let log = Log {
                            // TODO: fetch correct address from Kakarot.
                            // Contract Address is the account contract's address (EOA or KakarotAA)
                            address: H160::from_slice(&contract_address.0),
                            topics,
                            data: RpcBytes::from(data.0),
                            block_hash: None,
                            block_number: None,
                            transaction_hash: None,
                            transaction_index: None,
                            log_index: None,
                            removed: false,
                        };

                        tmp_logs.push(log);
                    }

                    res_receipt.logs = tmp_logs;
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
                            .safe_get_evm_address(&v0.contract_address, &starknet_block_id)
                            .await;
                        res_receipt.contract_address = Some(eth_address);
                    }
                    InvokeTransaction::V1(_) => {}
                };
            }
            StarknetTransaction::DeployAccount(_) | StarknetTransaction::Deploy(_) => {}
            _ => return Ok(None),
        };

        let eth_tx = self
            .starknet_tx_into_kakarot_tx(starknet_tx, None, None)
            .await;
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
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, KakarotClientError> {
        let request = FunctionCall {
            contract_address: *starknet_address,
            entry_point_selector: GET_EVM_ADDRESS,
            calldata: vec![],
        };

        let evm_address_felt = self.client.call(request, starknet_block_id).await?;
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

    /// Get the balance in Starknet's native token of a specific EVM address.
    /// Reproduces the principle of Kakarot native coin by using Starknet's native ERC20 token
    /// (gas-utility token) ### Arguments
    /// * `ethereum_address` - The EVM address to get the balance of
    /// * `block_id` - The block to get the balance at
    ///
    /// ### Returns
    /// * `Result<U256, KakarotClientError>` - The balance of the EVM address in Starknet's native
    ///   token
    async fn balance(
        &self,
        ethereum_address: Address,
        block_id: StarknetBlockId,
    ) -> Result<U256, KakarotClientError> {
        let starknet_address = self
            .compute_starknet_address(ethereum_address, &block_id)
            .await?;

        let request = FunctionCall {
            // This FieldElement::from_hex_be cannot fail as the value is a constant
            contract_address: FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(),
            entry_point_selector: BALANCE_OF,
            calldata: vec![starknet_address],
        };

        let balance_felt = self.client.call(request, block_id).await?;

        let balance = balance_felt
            .first()
            .ok_or_else(|| {
                KakarotClientError::OtherError(anyhow::anyhow!(
                    "Kakarot Core: Failed to get native token balance"
                ))
            })?
            .to_bytes_be();

        let balance = U256::from_be_bytes(balance);

        Ok(balance)
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
    async fn token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, KakarotClientError> {
        let entrypoint = hash_to_field_element(keccak256("balanceOf(address)")).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Failed to convert entrypoint to FieldElement: {}",
                e
            ))
        })?;
        let felt_address = FieldElement::from_str(&address.to_string()).map_err(|e| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Failed to convert address to FieldElement: {}",
                e
            ))
        })?;
        let handles = contract_addresses.into_iter().map(|token_address| {
            let calldata = vec![entrypoint, felt_address];

            self.call_view(
                token_address,
                Bytes::from(vec_felt_to_bytes(calldata).0),
                StarknetBlockId::Tag(BlockTag::Latest),
            )
        });
        let token_balances = join_all(handles)
            .await
            .into_iter()
            .map(|token_address| match token_address {
                Ok(call) => {
                    let hex_balance = U256::from_str_radix(&call.to_string(), 16)
                        .map_err(|e| {
                            KakarotClientError::OtherError(anyhow::anyhow!(
                                "Failed to convert token balance to U256: {}",
                                e
                            ))
                        })
                        .unwrap();
                    TokenBalance {
                        contract_address: address,
                        token_balance: Some(hex_balance),
                        error: None,
                    }
                }
                Err(e) => TokenBalance {
                    contract_address: address,
                    token_balance: None,
                    error: Some(format!("kakarot_getTokenBalances Error: {e}")),
                },
            })
            .collect();

        Ok(TokenBalances {
            address,
            token_balances,
        })
    }

    async fn starknet_tx_into_kakarot_tx(
        &self,
        tx: StarknetTransaction,
        block_hash: Option<H256>,
        block_number: Option<U256>,
    ) -> Result<EtherTransaction, KakarotClientError> {
        let mut ether_tx = EtherTransaction::default();
        let class_hash;
        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);
        let max_priority_fee_per_gas = Some(self.max_priority_fee_per_gas());

        match tx {
            StarknetTransaction::Invoke(invoke_tx) => {
                match invoke_tx {
                    InvokeTransaction::V0(v0) => {
                        // Extract relevant fields from InvokeTransactionV0 and convert them to the
                        // corresponding fields in EtherTransaction
                        ether_tx.hash = H256::from_slice(&v0.transaction_hash.to_bytes_be());
                        class_hash = self
                            .client
                            .get_class_hash_at(
                                StarknetBlockId::Tag(BlockTag::Latest),
                                v0.contract_address,
                            )
                            .await?;

                        ether_tx.nonce = felt_to_u256(v0.nonce);
                        ether_tx.from = self
                            .get_evm_address(&v0.contract_address, &starknet_block_id)
                            .await?;
                        // Define gas_price data
                        ether_tx.gas_price = None;
                        // Extracting the signature
                        let signature = decode_signature_from_tx_calldata(&v0.calldata)?;
                        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
                        ether_tx.signature = Some(Signature {
                            r: signature.r,
                            s: signature.s,
                            v: U256::from_limbs_slice(&[v]),
                        });
                        // Extracting the data (transform from calldata)
                        ether_tx.input = vec_felt_to_bytes(v0.calldata);
                        //TODO:  Fetch transaction To
                        ether_tx.to = None;
                        //TODO:  Fetch value
                        ether_tx.value = U256::from(100);
                        //TODO: Fetch Gas
                        ether_tx.gas = U256::from(100);
                        // Extracting the chain_id
                        ether_tx.chain_id = Some(CHAIN_ID.into());
                        ether_tx.block_hash = block_hash;
                        ether_tx.block_number = block_number;
                        ether_tx.max_priority_fee_per_gas = max_priority_fee_per_gas;
                    }

                    InvokeTransaction::V1(v1) => {
                        // Extract relevant fields from InvokeTransactionV0 and convert them to the
                        // corresponding fields in EtherTransaction

                        ether_tx.hash = H256::from_slice(&v1.transaction_hash.to_bytes_be());
                        class_hash = self
                            .client
                            .get_class_hash_at(
                                StarknetBlockId::Tag(BlockTag::Latest),
                                v1.sender_address,
                            )
                            .await?;

                        ether_tx.nonce = felt_to_u256(v1.nonce);

                        ether_tx.from = self
                            .get_evm_address(&v1.sender_address, &starknet_block_id)
                            .await?;

                        // Define gas_price data
                        ether_tx.gas_price = None;
                        // Extracting the signature
                        let signature = decode_signature_from_tx_calldata(&v1.calldata)?;
                        let v = if signature.odd_y_parity { 1 } else { 0 } + 35 + 2 * CHAIN_ID;
                        ether_tx.signature = Some(Signature {
                            r: signature.r,
                            s: signature.s,
                            v: U256::from_limbs_slice(&[v]),
                        });
                        // Extracting the data
                        ether_tx.input = vec_felt_to_bytes(v1.calldata);
                        ether_tx.to = None;
                        // Extracting the to address
                        // TODO: Get Data from Calldata
                        ether_tx.to = None;
                        // Extracting the value
                        ether_tx.value = U256::from(100);
                        // TODO:: Get Gas from Estimate
                        ether_tx.gas = U256::from(100);
                        // Extracting the chain_id
                        ether_tx.chain_id = Some(CHAIN_ID.into());
                        // Extracting the access_list
                        ether_tx.access_list = None;
                        // Extracting the transaction_type
                        ether_tx.transaction_type = None;
                        ether_tx.block_hash = block_hash;
                        ether_tx.block_number = block_number;
                        ether_tx.max_priority_fee_per_gas = max_priority_fee_per_gas;
                    }
                }
            }
            // Repeat the process for each variant of StarknetTransaction
            StarknetTransaction::L1Handler(_)
            | StarknetTransaction::Declare(_)
            | StarknetTransaction::Deploy(_)
            | StarknetTransaction::DeployAccount(_) => {
                return Err(KakarotClientError::OtherError(anyhow::anyhow!(
                    "Kakarot starknet_tx_into_eth_tx: L1Handler, Declare, Deploy and DeployAccount transactions unsupported"
                )));
            }
        }

        if class_hash == self.proxy_account_class_hash() {
            Ok(ether_tx)
        } else {
            Err(KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot Filter: Tx is not part of Kakarot"
            )))
        }
    }

    async fn starknet_block_to_eth_block(
        &self,
        block: MaybePendingStarknetBlock,
    ) -> Result<RichBlock, KakarotClientError> {
        // Fixed fields in the Ethereum block as Starknet does not have these fields

        //TODO: Fetch real data
        let gas_limit = U256::from(1_000_000);

        //TODO: Fetch real data
        let gas_used = U256::from(500_000);

        //TODO: Fetch real data
        let difficulty = U256::ZERO;

        //TODO: Fetch real data
        let nonce: Option<H64> = Some(H64::zero());

        //TODO: Fetch real data
        let size: Option<U256> = Some(U256::from(1_000_000));

        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data = Bytes::from(b"0x00");
        //TODO: Fetch real data
        let total_difficulty: U256 = U256::ZERO;
        //TODO: Fetch real data
        let base_fee_per_gas = self.base_fee_per_gas();
        //TODO: Fetch real data
        let mix_hash = H256::zero();

        match block {
            MaybePendingStarknetBlock::BlockWithTxHashes(maybe_pending_block) => {
                match maybe_pending_block {
                    MaybePendingBlockWithTxHashes::PendingBlock(pending_block_with_tx_hashes) => {
                        let parent_hash = H256::from_slice(
                            &pending_block_with_tx_hashes.parent_hash.to_bytes_be(),
                        );
                        let sequencer = starknet_address_to_ethereum_address(
                            &pending_block_with_tx_hashes.sequencer_address,
                        );
                        let timestamp = U256::from_be_bytes(
                            pending_block_with_tx_hashes.timestamp.to_be_bytes(),
                        );

                        //TODO: Add filter to tx_hashes
                        let transactions = BlockTransactions::Hashes(
                            pending_block_with_tx_hashes
                                .transactions
                                .into_iter()
                                .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                                .collect(),
                        );

                        let header = Header {
                            // PendingBlockWithTxHashes doesn't have a block hash
                            hash: None,
                            parent_hash,
                            uncles_hash: parent_hash,
                            miner: sequencer,
                            // PendingBlockWithTxHashes doesn't have a state root
                            state_root: H256::zero(),
                            // PendingBlockWithTxHashes doesn't have a transactions root
                            transactions_root: H256::zero(),
                            // PendingBlockWithTxHashes doesn't have a receipts root
                            receipts_root: H256::zero(),
                            // PendingBlockWithTxHashes doesn't have a block number
                            number: None,
                            gas_used,
                            gas_limit,
                            extra_data,
                            logs_bloom,
                            timestamp,
                            difficulty,
                            nonce,
                            mix_hash,
                            withdrawals_root: Some(H256::zero()),
                            base_fee_per_gas: Some(base_fee_per_gas),
                        };
                        let block = Block {
                            header,
                            total_difficulty: Some(total_difficulty),
                            uncles: vec![],
                            transactions,
                            size,
                            withdrawals: Some(vec![]),
                        };
                        Ok(Rich::<Block> {
                            inner: block,
                            extra_info: BTreeMap::default(),
                        })
                    }
                    MaybePendingBlockWithTxHashes::Block(block_with_tx_hashes) => {
                        let hash = H256::from_slice(&block_with_tx_hashes.block_hash.to_bytes_be());
                        let parent_hash =
                            H256::from_slice(&block_with_tx_hashes.parent_hash.to_bytes_be());

                        let sequencer = starknet_address_to_ethereum_address(
                            &block_with_tx_hashes.sequencer_address,
                        );

                        let state_root = H256::zero();
                        let number = U256::from(block_with_tx_hashes.block_number);
                        let timestamp = U256::from(block_with_tx_hashes.timestamp);
                        //TODO: Add filter to tx_hashes
                        let transactions = BlockTransactions::Hashes(
                            block_with_tx_hashes
                                .transactions
                                .into_iter()
                                .map(|tx| H256::from_slice(&tx.to_bytes_be()))
                                .collect(),
                        );
                        let header = Header {
                            hash: Some(hash),
                            parent_hash,
                            uncles_hash: parent_hash,
                            miner: sequencer,
                            state_root,
                            // BlockWithTxHashes doesn't have a transactions root
                            transactions_root: H256::zero(),
                            // BlockWithTxHashes doesn't have a receipts root
                            receipts_root: H256::zero(),
                            number: Some(number),
                            gas_used,
                            gas_limit,
                            extra_data,
                            logs_bloom,
                            timestamp,
                            difficulty,
                            nonce,
                            mix_hash,
                            withdrawals_root: Some(H256::zero()),
                            base_fee_per_gas: Some(base_fee_per_gas),
                        };
                        let block = Block {
                            header,
                            total_difficulty: Some(total_difficulty),
                            uncles: vec![],
                            transactions,
                            size,
                            withdrawals: Some(vec![]),
                        };
                        Ok(Rich::<Block> {
                            inner: block,
                            extra_info: BTreeMap::default(),
                        })
                    }
                }
            }
            MaybePendingStarknetBlock::BlockWithTxs(maybe_pending_block) => {
                match maybe_pending_block {
                    MaybePendingBlockWithTxs::PendingBlock(pending_block_with_txs) => {
                        let parent_hash =
                            H256::from_slice(&pending_block_with_txs.parent_hash.to_bytes_be());

                        let sequencer = starknet_address_to_ethereum_address(
                            &pending_block_with_txs.sequencer_address,
                        );

                        let timestamp =
                            U256::from_be_bytes(pending_block_with_txs.timestamp.to_be_bytes());

                        let transactions = self
                            .filter_starknet_into_eth_txs(
                                pending_block_with_txs.transactions,
                                None,
                                None,
                            )
                            .await?;
                        let header = Header {
                            // PendingBlockWithTxs doesn't have a block hash
                            hash: None,
                            parent_hash,
                            uncles_hash: parent_hash,
                            miner: sequencer,
                            // PendingBlockWithTxs doesn't have a state root
                            state_root: H256::zero(),
                            // PendingBlockWithTxs doesn't have a transactions root
                            transactions_root: H256::zero(),
                            // PendingBlockWithTxs doesn't have a receipts root
                            receipts_root: H256::zero(),
                            // PendingBlockWithTxs doesn't have a block number
                            number: None,
                            gas_used,
                            gas_limit,
                            extra_data,
                            logs_bloom,
                            timestamp,
                            difficulty,
                            nonce,
                            base_fee_per_gas: Some(base_fee_per_gas),
                            mix_hash,
                            withdrawals_root: Some(H256::zero()),
                        };
                        let block = Block {
                            header,
                            total_difficulty: Some(total_difficulty),
                            uncles: vec![],
                            transactions,
                            size,
                            withdrawals: Some(vec![]),
                        };
                        Ok(Rich::<Block> {
                            inner: block,
                            extra_info: BTreeMap::default(),
                        })
                    }
                    MaybePendingBlockWithTxs::Block(block_with_txs) => {
                        let hash = H256::from_slice(&block_with_txs.block_hash.to_bytes_be());
                        let parent_hash =
                            H256::from_slice(&block_with_txs.parent_hash.to_bytes_be());

                        let sequencer =
                            starknet_address_to_ethereum_address(&block_with_txs.sequencer_address);

                        let state_root = H256::zero();
                        let transactions_root = H256::zero();
                        let receipts_root = H256::zero();

                        let number = U256::from(block_with_txs.block_number);
                        let timestamp = U256::from(block_with_txs.timestamp);

                        let blockhash_opt =
                            Some(H256::from_slice(&(block_with_txs.block_hash).to_bytes_be()));
                        let blocknum_opt = Some(U256::from(block_with_txs.block_number));

                        let transactions = self
                            .filter_starknet_into_eth_txs(
                                block_with_txs.transactions,
                                blockhash_opt,
                                blocknum_opt,
                            )
                            .await?;

                        let header = Header {
                            hash: Some(hash),
                            parent_hash,
                            uncles_hash: parent_hash,
                            miner: sequencer,
                            state_root,
                            // BlockWithTxHashes doesn't have a transactions root
                            transactions_root,
                            // BlockWithTxHashes doesn't have a receipts root
                            receipts_root,
                            number: Some(number),
                            gas_used,
                            gas_limit,
                            extra_data,
                            logs_bloom,
                            timestamp,
                            difficulty,
                            nonce,
                            base_fee_per_gas: Some(base_fee_per_gas),
                            mix_hash,
                            withdrawals_root: Some(H256::zero()),
                        };
                        let block = Block {
                            header,
                            total_difficulty: Some(total_difficulty),
                            uncles: vec![],
                            transactions,
                            size,
                            withdrawals: Some(vec![]),
                        };
                        Ok(Rich::<Block> {
                            inner: block,
                            extra_info: BTreeMap::default(),
                        })
                    }
                }
            }
        }
    }

    async fn filter_starknet_into_eth_txs(
        &self,
        initial_transactions: Vec<StarknetTransaction>,
        blockhash_opt: Option<H256>,
        blocknum_opt: Option<U256>,
    ) -> Result<BlockTransactions, KakarotClientError> {
        let handles = initial_transactions
            .into_iter()
            .map(|starknet_transaction| {
                self.starknet_tx_into_kakarot_tx(starknet_transaction, blockhash_opt, blocknum_opt)
            });
        let transactions_vec = join_all(handles)
            .await
            .into_iter()
            .filter_map(|transaction| transaction.ok())
            .collect();
        Ok(BlockTransactions::Full(transactions_vec))
    }

    async fn send_transaction(&self, bytes: Bytes) -> Result<H256, KakarotClientError> {
        let mut data = bytes.as_ref();

        if data.is_empty() {
            return Err(KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot send_transaction: Transaction bytes are empty"
            )));
        };

        let transaction = TransactionSigned::decode(&mut data).map_err(|_| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot send_transaction: transaction bytes failed to be decoded"
            ))
        })?;

        let evm_address = transaction.recover_signer().ok_or_else(|| {
            KakarotClientError::OtherError(anyhow::anyhow!(
                "Kakarot send_transaction: signature ecrecover failed"
            ))
        })?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        let starknet_address = self
            .compute_starknet_address(evm_address, &starknet_block_id)
            .await?;

        let nonce = FieldElement::from(transaction.nonce());

        let calldata = raw_starknet_calldata(self.kakarot_address, bytes);

        // Get estimated_fee from Starknet
        let max_fee = FieldElement::from(100_000_000_000_000_000_u64);

        let signature = vec![];

        let request = BroadcastedInvokeTransactionV1 {
            max_fee,
            signature,
            nonce,
            sender_address: starknet_address,
            calldata,
        };

        let starknet_transaction_hash = self.submit_starknet_transaction(request).await?;

        Ok(starknet_transaction_hash)
    }

    /// Returns the fixed base_fee_per_gas of Kakarot
    /// Since Starknet works on a FCFS basis (FIFO queue), it is not possible to tip miners to
    /// incentivize faster transaction inclusion
    /// As a result, in Kakarot, gas_price := base_fee_per_gas
    ///
    /// Computes the Starknet gas_price using starknet_estimateFee RPC method
    fn base_fee_per_gas(&self) -> U256 {
        U256::from(BASE_FEE_PER_GAS)
    }

    fn max_priority_fee_per_gas(&self) -> U128 {
        MAX_PRIORITY_FEE_PER_GAS
    }

    async fn fee_history(
        &self,
        _block_count: U256,
        _newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory, KakarotClientError> {
        let block_count_usize = usize::from_str_radix(&_block_count.to_string(), 16).unwrap_or(1);

        let base_fee = self.base_fee_per_gas();

        let base_fee_per_gas: Vec<U256> = vec![base_fee; block_count_usize + 1];
        let newest_block = match _newest_block {
            BlockNumberOrTag::Number(n) => n,
            // TODO: Add Genesis block number
            BlockNumberOrTag::Earliest => 1_u64,
            _ => self.block_number().await?.as_u64(),
        };

        let gas_used_ratio: Vec<f64> = vec![0.9; block_count_usize];
        let oldest_block: U256 = U256::from(newest_block) - _block_count;

        Ok(FeeHistory {
            base_fee_per_gas,
            gas_used_ratio,
            oldest_block,
            reward: None,
        })
    }

    async fn estimate_gas(
        &self,
        _call_request: CallRequest,
        _block_number: Option<BlockId>,
    ) -> Result<U256, KakarotClientError> {
        todo!();
    }
}
