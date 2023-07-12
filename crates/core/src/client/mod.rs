pub mod api;
pub mod config;
pub mod constants;
pub mod errors;
pub mod helpers;
#[cfg(test)]
pub mod tests;

use async_trait::async_trait;
use eyre::Result;
use futures::future::join_all;
use helpers::{raw_starknet_calldata, vec_felt_to_bytes};
use reth_primitives::{
    keccak256, Address, BlockId, BlockNumberOrTag, Bloom, Bytes, TransactionSigned, H256, U128, U256, U64, U8,
};
use reth_rlp::Decodable;
use reth_rpc_types::{
    BlockTransactions, CallRequest, FeeHistory, Index, RichBlock, SyncInfo, SyncStatus,
    Transaction as EtherTransaction, TransactionReceipt,
};
use starknet::core::types::{
    BlockId as StarknetBlockId, BlockTag, BroadcastedInvokeTransaction, BroadcastedInvokeTransactionV1, FieldElement,
    FunctionCall, InvokeTransactionReceipt, MaybePendingBlockWithTxs, MaybePendingTransactionReceipt, StarknetError,
    SyncStatusType, Transaction as TransactionType, TransactionReceipt as StarknetTransactionReceipt,
    TransactionStatus as StarknetTransactionStatus,
};
use starknet::providers::{Provider, ProviderError};

use self::api::{KakarotEthApi, KakarotStarknetApi};
use self::config::StarknetConfig;
use self::constants::gas::{BASE_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use self::constants::selectors::{BALANCE_OF, BYTECODE, EVM_CONTRACT_DEPLOYED, GET_EVM_ADDRESS};
use self::constants::{MAX_FEE, STARKNET_NATIVE_TOKEN};
use self::errors::EthApiError;
use self::helpers::DataDecodingError;
use crate::kakarot::KakarotContract;
use crate::models::balance::{TokenBalance, TokenBalances};
use crate::models::block::{BlockWithTxHashes, BlockWithTxs, EthBlockId};
use crate::models::convertible::{ConvertibleStarknetBlock, ConvertibleStarknetEvent, ConvertibleStarknetTransaction};
use crate::models::event::StarknetEvent;
use crate::models::felt::Felt252Wrapper;
use crate::models::transaction::{StarknetTransaction, StarknetTransactions};
use crate::models::ConversionError;

pub struct KakarotClient<P: Provider + Send + Sync> {
    starknet_provider: P,
    kakarot_contract: KakarotContract<P>,
}

impl<P: Provider + Send + Sync> KakarotClient<P> {
    /// Create a new `KakarotClient`.
    ///
    /// # Arguments
    ///
    /// * `starknet_config(StarknetConfig)` - `StarkNet` configuration.
    /// * `provider(T)` - `StarkNet` provider.
    ///
    /// # Errors
    ///
    /// `Err(EthApiError<T>)` if the operation failed.
    pub fn new(starknet_config: StarknetConfig, starknet_provider: P) -> Self {
        let StarknetConfig { kakarot_address, proxy_account_class_hash, .. } = starknet_config;
        let kakarot_contract = KakarotContract::new(kakarot_address, proxy_account_class_hash);

        Self { starknet_provider, kakarot_contract }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> KakarotEthApi<P> for KakarotClient<P> {
    /// Returns the latest block number
    async fn block_number(&self) -> Result<U64, EthApiError<P::Error>> {
        let block_number = self.starknet_provider.block_number().await?;
        Ok(block_number.into())
    }

    /// Returns the bytecode of a contract given its address and a block id.
    async fn get_code(&self, ethereum_address: Address, block_id: BlockId) -> Result<Bytes, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;

        // Convert the hex-encoded string to a FieldElement
        let ethereum_address: Felt252Wrapper = ethereum_address.into();
        let ethereum_address = ethereum_address.into();

        let starknet_contract_address = self
            .kakarot_contract
            .compute_starknet_address(&self.starknet_provider, &ethereum_address, &starknet_block_id)
            .await?;

        // Prepare the calldata for the bytecode function call
        let request = FunctionCall {
            contract_address: starknet_contract_address,
            entry_point_selector: BYTECODE,
            calldata: vec![],
        };

        // Make the function call to get the contract bytecode
        let bytecode = self.starknet_provider.call(request, starknet_block_id).await.or_else(|err| match err {
            ProviderError::StarknetError(starknet_error) => match starknet_error {
                // TODO: we just need to test against ContractNotFound but madara is currently returning the wrong
                // error See https://github.com/keep-starknet-strange/madara/issues/853
                StarknetError::ContractError | StarknetError::ContractNotFound => Ok(vec![]),
                _ => Err(EthApiError::from(err)),
            },
            _ => Err(EthApiError::from(err)),
        })?;

        // Convert the result of the function call to a vector of bytes
        let bytecode: Bytes = vec_felt_to_bytes(bytecode);
        Ok(bytecode)
    }

    /// Returns the result of executing a call on a ethereum address for a given calldata and block
    /// without creating a transaction.
    async fn call_view(
        &self,
        ethereum_address: Address,
        calldata: Bytes,
        block_id: BlockId,
    ) -> Result<Bytes, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;

        let ethereum_address: Felt252Wrapper = ethereum_address.into();
        let ethereum_address = ethereum_address.into();

        let mut calldata = calldata.clone().into_iter().map(FieldElement::from).collect::<Vec<_>>();

        let result = self
            .kakarot_contract
            .eth_call(&self.starknet_provider, &ethereum_address, &mut calldata, &starknet_block_id)
            .await?;

        Ok(result)
    }

    /// Get the syncing status of the light client
    async fn syncing(&self) -> Result<SyncStatus, EthApiError<P::Error>> {
        let status = self.starknet_provider.syncing().await?;

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
    async fn block_transaction_count_by_number(&self, number: BlockNumberOrTag) -> Result<U64, EthApiError<P::Error>> {
        let block_id = BlockId::Number(number);
        self.get_transaction_count_by_block(block_id).await
    }

    /// Get the number of transactions in a block given a block hash.
    async fn block_transaction_count_by_hash(&self, hash: H256) -> Result<U64, EthApiError<P::Error>> {
        let block_id = BlockId::Hash(hash.into());
        self.get_transaction_count_by_block(block_id).await
    }

    /// Returns the number of transactions in a block given a block id.
    async fn get_transaction_count_by_block(&self, block_id: BlockId) -> Result<U64, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;
        let starknet_block = self.starknet_provider.get_block_with_txs(starknet_block_id).await?;

        let block_transactions = match starknet_block {
            MaybePendingBlockWithTxs::PendingBlock(pending_block_with_txs) => {
                self.filter_starknet_into_eth_txs(pending_block_with_txs.transactions.into(), None, None).await
            }
            MaybePendingBlockWithTxs::Block(block_with_txs) => {
                let block_hash: Felt252Wrapper = block_with_txs.block_hash.into();
                let block_hash = Some(block_hash.into());
                let block_number: Felt252Wrapper = block_with_txs.block_number.into();
                let block_number = Some(block_number.into());
                self.filter_starknet_into_eth_txs(block_with_txs.transactions.into(), block_hash, block_number).await
            }
        };
        let len = match block_transactions {
            BlockTransactions::Full(transactions) => transactions.len(),
            BlockTransactions::Hashes(_) => 0,
            BlockTransactions::Uncle => 0,
        };
        Ok(U64::from(len))
    }

    /// Returns the transaction for a given block id and transaction index.
    async fn transaction_by_block_id_and_index(
        &self,
        block_id: BlockId,
        tx_index: Index,
    ) -> Result<EtherTransaction, EthApiError<P::Error>> {
        let index: u64 = usize::from(tx_index) as u64;
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;

        let starknet_tx: StarknetTransaction =
            self.starknet_provider.get_transaction_by_block_id_and_index(starknet_block_id, index).await?.into();

        let tx_hash: FieldElement = starknet_tx.transaction_hash()?.into();

        let tx_receipt = self.starknet_provider.get_transaction_receipt(tx_hash).await?;
        let (block_hash, block_num) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => {
                let block_hash: Felt252Wrapper = tr.block_hash.into();
                (Some(block_hash.into()), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };

        let eth_tx = starknet_tx.to_eth_transaction(self, block_hash, block_num, Some(U256::from(index))).await?;
        Ok(eth_tx)
    }

    /// Returns the transaction for a given transaction hash.
    async fn transaction_by_hash(&self, hash: H256) -> Result<EtherTransaction, EthApiError<P::Error>> {
        let hash: Felt252Wrapper = hash.try_into()?;
        let hash: FieldElement = hash.into();

        let transaction: StarknetTransaction = self.starknet_provider.get_transaction_by_hash(hash).await?.into();
        let tx_receipt = self.starknet_provider.get_transaction_receipt(hash).await?;
        let (block_hash, block_num) = match tx_receipt {
            MaybePendingTransactionReceipt::Receipt(StarknetTransactionReceipt::Invoke(tr)) => {
                let block_hash: Felt252Wrapper = tr.block_hash.into();
                (Some(block_hash.into()), Some(U256::from(tr.block_number)))
            }
            _ => (None, None), // skip all transactions other than Invoke, covers the pending case
        };
        let eth_transaction = transaction.to_eth_transaction(self, block_hash, block_num, None).await?;
        Ok(eth_transaction)
    }

    /// Returns the receipt of a transaction by transaction hash.
    async fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>, EthApiError<P::Error>> {
        // TODO: Error when trying to transform 32 bytes hash to FieldElement
        let transaction_hash: Felt252Wrapper = hash.try_into()?;
        let starknet_tx_receipt =
            self.starknet_provider.get_transaction_receipt::<FieldElement>(transaction_hash.into()).await?;

        let res_receipt = match starknet_tx_receipt {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                StarknetTransactionReceipt::Invoke(InvokeTransactionReceipt {
                    transaction_hash,
                    status,
                    block_hash,
                    block_number,
                    events,
                    ..
                }) => {
                    let starknet_tx: StarknetTransaction =
                        self.starknet_provider.get_transaction_by_hash(transaction_hash).await?.into();

                    let transaction_hash: Felt252Wrapper = transaction_hash.into();
                    let transaction_hash: Option<H256> = Some(transaction_hash.into());

                    let block_hash: Felt252Wrapper = block_hash.into();
                    let block_hash: Option<H256> = Some(block_hash.into());

                    let block_number: Felt252Wrapper = block_number.into();
                    let block_number: Option<U256> = Some(block_number.into());

                    let eth_tx = starknet_tx.to_eth_transaction(self, None, None, None).await?;
                    let from = eth_tx.from;
                    let to = eth_tx.to;
                    let contract_address = match to {
                        // If to is Some, means contract_address should be None as it is a normal transaction
                        Some(_) => None,
                        // If to is None, is a contract creation transaction so contract_address should be Some
                        None => {
                            let event = events
                                .iter()
                                .find(|event| event.keys.iter().any(|key| *key == EVM_CONTRACT_DEPLOYED))
                                .ok_or(EthApiError::Other(anyhow::anyhow!(
                                    "Kakarot Core: No contract deployment event found in Kakarot transaction receipt"
                                )))?;

                            let evm_address =
                                event.data.first().ok_or(DataDecodingError::InvalidReturnArrayLength {
                                    entrypoint: "deployment".into(),
                                    expected: 1,
                                    actual: 0,
                                })?;

                            let evm_address = Felt252Wrapper::from(*evm_address);
                            Some(evm_address.try_into()?)
                        }
                    };

                    let status_code = match status {
                        StarknetTransactionStatus::Rejected | StarknetTransactionStatus::Pending => Some(U64::from(0)),
                        StarknetTransactionStatus::AcceptedOnL1 | StarknetTransactionStatus::AcceptedOnL2 => {
                            Some(U64::from(1))
                        }
                    };

                    let logs = events
                        .into_iter()
                        .map(StarknetEvent::new)
                        .filter_map(|event| {
                            event.to_eth_log(self, block_hash, block_number, transaction_hash, None, None).ok()
                        })
                        .collect();

                    TransactionReceipt {
                        transaction_hash,
                        // TODO: transition this hardcoded default out of nearing-demo-day hack and seeing how to
                        // properly source/translate this value
                        transaction_index: Some(U256::ZERO),
                        block_hash,
                        block_number,
                        from,
                        to,
                        cumulative_gas_used: U256::from(1_000_000), // TODO: Fetch real data
                        gas_used: Some(U256::from(500_000)),
                        contract_address,
                        logs,
                        state_root: None,             // TODO: Fetch real data
                        logs_bloom: Bloom::default(), // TODO: Fetch real data
                        status_code,
                        effective_gas_price: U128::from(1_000_000), // TODO: Fetch real data
                        transaction_type: U8::from(0),              // TODO: Fetch real data
                    }
                }
                // L1Handler, Declare, Deploy and DeployAccount transactions unsupported for now in
                // Kakarot
                _ => return Ok(None),
            },
            MaybePendingTransactionReceipt::PendingReceipt(_) => {
                return Ok(None);
            }
        };

        Ok(Some(res_receipt))
    }

    /// Returns the nonce for a given ethereum address
    /// if ethereum -> stark mapping doesn't exist in the starknet provider, we translate
    /// ContractNotFound errors into zeros
    async fn nonce(&self, ethereum_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;
        let starknet_address = self.compute_starknet_address(ethereum_address, &starknet_block_id).await?;

        self.starknet_provider
            .get_nonce(starknet_block_id, starknet_address)
            .await
            .map(|nonce| {
                let nonce: Felt252Wrapper = nonce.into();
                nonce.into()
            })
            .or_else(|err| match err {
                ProviderError::StarknetError(StarknetError::ContractNotFound) => Ok(U256::from(0)),
                _ => Err(EthApiError::from(err)),
            })
    }

    /// Returns the balance in Starknet's native token of a specific EVM address.
    async fn balance(&self, ethereum_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;
        let starknet_address = self.compute_starknet_address(ethereum_address, &starknet_block_id).await?;

        let request = FunctionCall {
            // This FieldElement::from_hex_be cannot fail as the value is a constant
            contract_address: FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap(),
            entry_point_selector: BALANCE_OF,
            calldata: vec![starknet_address],
        };

        let balance = self.starknet_provider.call(request, starknet_block_id).await?;

        let balance: Felt252Wrapper = (*balance.first().ok_or_else(|| {
            DataDecodingError::InvalidReturnArrayLength { entrypoint: "balance".into(), expected: 1, actual: 0 }
        })?)
        .into();

        Ok(balance.into())
    }

    /// Returns token balances for a specific address given a list of contracts addresses.
    async fn token_balances(
        &self,
        address: Address,
        contract_addresses: Vec<Address>,
    ) -> Result<TokenBalances, EthApiError<P::Error>> {
        let entrypoint: Felt252Wrapper = keccak256("balanceOf(address)").try_into()?;
        let entrypoint: FieldElement = entrypoint.into();

        let addr: Felt252Wrapper = address.into();
        let addr: FieldElement = addr.into();

        let handles = contract_addresses.into_iter().map(|token_address| {
            let calldata = vec![entrypoint, addr];

            self.call_view(
                token_address,
                Bytes::from(vec_felt_to_bytes(calldata).0),
                BlockId::from(BlockNumberOrTag::Latest),
            )
        });
        let token_balances = join_all(handles)
            .await
            .into_iter()
            .map(|token_address| match token_address {
                Ok(call) => {
                    let balance = U256::try_from_be_slice(call.as_ref())
                        .ok_or(ConversionError::Other("error converting from Bytes to U256".into()))
                        .unwrap();
                    TokenBalance { contract_address: address, token_balance: Some(balance), error: None }
                }
                Err(e) => TokenBalance {
                    contract_address: address,
                    token_balance: None,
                    error: Some(format!("kakarot_getTokenBalances Error: {e}")),
                },
            })
            .collect();

        Ok(TokenBalances { address, token_balances })
    }

    /// Sends raw Ethereum transaction bytes to Kakarot
    async fn send_transaction(&self, bytes: Bytes) -> Result<H256, EthApiError<P::Error>> {
        let mut data = bytes.as_ref();

        let transaction = TransactionSigned::decode(&mut data).map_err(DataDecodingError::TransactionDecodingError)?;

        let evm_address = transaction.recover_signer().ok_or_else(|| {
            EthApiError::Other(anyhow::anyhow!("Kakarot send_transaction: signature ecrecover failed"))
        })?;

        let starknet_block_id = StarknetBlockId::Tag(BlockTag::Latest);

        let starknet_address = self.compute_starknet_address(evm_address, &starknet_block_id).await?;

        let nonce = FieldElement::from(transaction.nonce());

        let calldata = raw_starknet_calldata(self.kakarot_address(), bytes);

        // Get estimated_fee from Starknet
        let max_fee = *MAX_FEE;

        let signature = vec![];

        let request =
            BroadcastedInvokeTransactionV1 { max_fee, signature, nonce, sender_address: starknet_address, calldata };

        let starknet_transaction_hash = self.submit_starknet_transaction(request).await?;

        Ok(starknet_transaction_hash)
    }

    /// Returns the fixed base_fee_per_gas of Kakarot
    /// Since Starknet works on a FCFS basis (FIFO queue), it is not possible to tip miners to
    /// incentivize faster transaction inclusion
    /// As a result, in Kakarot, gas_price := base_fee_per_gas
    fn base_fee_per_gas(&self) -> U256 {
        U256::from(BASE_FEE_PER_GAS)
    }

    /// Returns the max_priority_fee_per_gas of Kakarot
    fn max_priority_fee_per_gas(&self) -> U128 {
        MAX_PRIORITY_FEE_PER_GAS
    }

    /// Returns the fee history of Kakarot ending at the newest block and going back `block_count`
    async fn fee_history(
        &self,
        block_count: U256,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> Result<FeeHistory, EthApiError<P::Error>> {
        let block_count_usize =
            usize::try_from(block_count).map_err(|e| ConversionError::ValueOutOfRange(e.to_string()))?;

        let base_fee = self.base_fee_per_gas();

        let base_fee_per_gas: Vec<U256> = vec![base_fee; block_count_usize + 1];
        let newest_block = match newest_block {
            BlockNumberOrTag::Number(n) => n,
            // TODO: Add Genesis block number
            BlockNumberOrTag::Earliest => 1_u64,
            _ => self.block_number().await?.as_u64(),
        };

        let gas_used_ratio: Vec<f64> = vec![0.9; block_count_usize];
        let newest_block = U256::from(newest_block);
        let oldest_block: U256 = if newest_block >= block_count { newest_block - block_count } else { U256::from(0) };

        // TODO: transition `reward` hardcoded default out of nearing-demo-day hack and seeing how to
        // properly source/translate this value
        Ok(FeeHistory { base_fee_per_gas, gas_used_ratio, oldest_block, reward: Some(vec![vec![U256::ZERO]]) })
    }

    /// Returns the estimated gas for a transaction
    async fn estimate_gas(
        &self,
        _call_request: CallRequest,
        _block_number: Option<BlockId>,
    ) -> Result<U256, EthApiError<P::Error>> {
        todo!();
    }
}

#[async_trait]
impl<P: Provider + Send + Sync> KakarotStarknetApi<P> for KakarotClient<P> {
    /// Returns the Kakarot contract address.
    fn kakarot_address(&self) -> FieldElement {
        self.kakarot_contract.address
    }

    /// Returns the Kakarot proxy account class hash.
    fn proxy_account_class_hash(&self) -> FieldElement {
        self.kakarot_contract.proxy_account_class_hash
    }

    /// Returns a reference to the Starknet provider.
    fn starknet_provider(&self) -> &P {
        &self.starknet_provider
    }

    /// Returns the EVM address associated with a given Starknet address for a given block id
    /// by calling the `get_evm_address` function on the Kakarot contract.
    async fn get_evm_address(
        &self,
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, EthApiError<P::Error>> {
        let request = FunctionCall {
            contract_address: *starknet_address,
            entry_point_selector: GET_EVM_ADDRESS,
            calldata: vec![],
        };

        let evm_address = self.starknet_provider.call(request, starknet_block_id).await?;
        let evm_address: Felt252Wrapper = (*evm_address.first().ok_or_else(|| {
            DataDecodingError::InvalidReturnArrayLength { entrypoint: "get_evm_address".into(), expected: 1, actual: 0 }
        })?)
        .into();

        Ok(evm_address.troncate_to_ethereum_address())
    }

    /// Submits a Kakarot transaction to the Starknet provider.
    async fn submit_starknet_transaction(
        &self,
        request: BroadcastedInvokeTransactionV1,
    ) -> Result<H256, EthApiError<P::Error>> {
        let transaction_result =
            self.starknet_provider.add_invoke_transaction(&BroadcastedInvokeTransaction::V1(request)).await?;

        Ok(H256::from(transaction_result.transaction_hash.to_bytes_be()))
    }

    /// Returns the EVM address associated with a given Starknet address for a given block id
    /// by calling the `compute_starknet_address` function on the Kakarot contract.
    async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        let ethereum_address: Felt252Wrapper = ethereum_address.into();
        let ethereum_address = ethereum_address.into();

        Ok(self
            .kakarot_contract
            .compute_starknet_address(&self.starknet_provider, &ethereum_address, starknet_block_id)
            .await?)
    }

    /// Returns the Ethereum transactions executed by the Kakarot contract by filtering the provided
    /// Starknet transaction.
    async fn filter_starknet_into_eth_txs(
        &self,
        initial_transactions: StarknetTransactions,
        block_hash: Option<H256>,
        block_number: Option<U256>,
    ) -> BlockTransactions {
        let handles = Into::<Vec<TransactionType>>::into(initial_transactions).into_iter().map(|tx| async move {
            let tx = Into::<StarknetTransaction>::into(tx);
            tx.to_eth_transaction(self, block_hash, block_number, None).await
        });
        let transactions_vec = join_all(handles).await.into_iter().filter_map(|transaction| transaction.ok()).collect();
        BlockTransactions::Full(transactions_vec)
    }

    /// Get the Kakarot eth block provided a Starknet block id.
    async fn get_eth_block_from_starknet_block(
        &self,
        block_id: StarknetBlockId,
        hydrated_tx: bool,
    ) -> Result<RichBlock, EthApiError<P::Error>> {
        if hydrated_tx {
            let block = self.starknet_provider.get_block_with_txs(block_id).await?;
            let starknet_block = BlockWithTxs::new(block);
            Ok(starknet_block.to_eth_block(self).await)
        } else {
            let block = self.starknet_provider.get_block_with_tx_hashes(block_id).await?;
            let starknet_block = BlockWithTxHashes::new(block);
            Ok(starknet_block.to_eth_block(self).await)
        }
    }
}
