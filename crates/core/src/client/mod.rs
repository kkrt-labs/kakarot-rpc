pub mod config;
pub mod constants;
pub mod errors;
pub mod helpers;
#[cfg(test)]
pub mod tests;
pub mod waiter;

use std::sync::Arc;

use eyre::Result;
use futures::future::join_all;
use reqwest::Client;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, H256, U128, U256, U64};
use reth_rpc_types::{BlockTransactions, RichBlock};
use starknet::accounts::SingleOwnerAccount;
use starknet::core::types::{
    BlockId as StarknetBlockId, BroadcastedInvokeTransaction, EmittedEvent, EventFilterWithPage, EventsPage,
    FieldElement, MaybePendingBlockWithTxHashes, MaybePendingBlockWithTxs, StarknetError,
    Transaction as TransactionType,
};
use starknet::providers::sequencer::models::{FeeEstimate, FeeUnit, TransactionSimulationInfo, TransactionTrace};
use starknet::providers::{MaybeUnknownErrorCode, Provider, ProviderError, StarknetErrorWithMessage};
use starknet::signers::LocalWallet;

use self::config::{KakarotRpcConfig, Network};
use self::constants::gas::{BASE_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use self::constants::{ESTIMATE_GAS, MAX_FEE};
use self::errors::EthApiError;
use self::waiter::TransactionWaiter;
use crate::contracts::account::{Account, KakarotAccount};
use crate::contracts::contract_account::ContractAccount;
use crate::contracts::erc20::ethereum_erc20::EthereumErc20;
use crate::contracts::kakarot::KakarotContract;
use crate::models::balance::{FutureTokenBalance, TokenBalances};
use crate::models::block::{BlockWithTxHashes, BlockWithTxs, EthBlockId};
use crate::models::convertible::{ConvertibleStarknetBlock, ConvertibleStarknetTransaction};
use crate::models::felt::Felt252Wrapper;
use crate::models::transaction::{StarknetTransaction, StarknetTransactions};
use crate::models::ConversionError;

pub struct KakarotClient<P: Provider + Send + Sync + 'static> {
    starknet_provider: Arc<P>,
    deployer_account: SingleOwnerAccount<Arc<P>, LocalWallet>,
    kakarot_contract: Arc<KakarotContract<P>>,
    network: Network,
}

impl<P: Provider + Send + Sync + 'static> KakarotClient<P> {
    /// Create a new `KakarotClient`.
    pub fn new(
        starknet_config: KakarotRpcConfig,
        starknet_provider: Arc<P>,
        starknet_account: SingleOwnerAccount<Arc<P>, LocalWallet>,
    ) -> Self {
        let KakarotRpcConfig {
            kakarot_address,
            proxy_account_class_hash,
            externally_owned_account_class_hash,
            contract_account_class_hash,
            network,
        } = starknet_config;

        let kakarot_contract = Arc::new(KakarotContract::new(
            starknet_provider.clone(),
            kakarot_address,
            proxy_account_class_hash,
            externally_owned_account_class_hash,
            contract_account_class_hash,
        ));

        Self { starknet_provider, network, kakarot_contract, deployer_account: starknet_account }
    }
    /// Returns the number of transactions in a block given a block id.
    pub async fn transaction_count_by_block(&self, block_id: BlockId) -> Result<U64, EthApiError<P::Error>> {
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

    /// Returns the nonce for a given ethereum address
    /// if it's an EOA, use native nonce and if it's a contract account, use managed nonce
    /// if ethereum -> stark mapping doesn't exist in the starknet provider, we translate
    /// ContractNotFound errors into zeros
    pub async fn nonce(&self, ethereum_address: Address, block_id: BlockId) -> Result<U256, EthApiError<P::Error>> {
        let starknet_block_id: StarknetBlockId = EthBlockId::new(block_id).try_into()?;
        let starknet_address = self.compute_starknet_address(ethereum_address, &starknet_block_id).await?;

        // Get the implementation of the account
        let account = KakarotAccount::new(starknet_address, self.starknet_provider());
        let class_hash = match account.implementation(&starknet_block_id).await {
            Ok(class_hash) => class_hash,
            Err(err) => match err {
                EthApiError::RequestError(ProviderError::StarknetError(StarknetErrorWithMessage {
                    code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                    ..
                })) => return Ok(U256::from(0)), // Return 0 if the account doesn't exist
                _ => return Err(err), // Propagate the error
            },
        };

        if class_hash == self.kakarot_contract.contract_account_class_hash {
            // Get the nonce of the contract account
            let contract_account = ContractAccount::new(starknet_address, self.starknet_provider());
            contract_account.nonce(&starknet_block_id).await
        } else {
            // Get the nonce of the EOA
            let nonce = self.starknet_provider.get_nonce(starknet_block_id, starknet_address).await?;
            Ok(Felt252Wrapper::from(nonce).into())
        }
    }

    /// Returns token balances for a specific address given a list of contracts addresses.
    pub async fn token_balances(
        &self,
        address: Address,
        token_addresses: Vec<Address>,
    ) -> Result<TokenBalances, EthApiError<P::Error>> {
        let block_id = BlockId::Number(BlockNumberOrTag::Latest);
        let kakarot_contract = self.kakarot_contract.clone();

        let handles = token_addresses.into_iter().map(|token_address| {
            let token_addr: Felt252Wrapper = token_address.into();
            let token = EthereumErc20::new(token_addr.into(), kakarot_contract.clone());

            FutureTokenBalance::<P, _>::new(token.balance_of(address.into(), block_id), token_address)
        });

        let token_balances = join_all(handles).await;

        Ok(TokenBalances { address, token_balances })
    }

    /// Returns the fixed base_fee_per_gas of Kakarot
    /// Since Starknet works on a FCFS basis (FIFO queue), it is not possible to tip miners to
    /// incentivize faster transaction inclusion
    /// As a result, in Kakarot, gas_price := base_fee_per_gas
    pub fn base_fee_per_gas(&self) -> U256 {
        U256::from(BASE_FEE_PER_GAS)
    }

    /// Returns the max_priority_fee_per_gas of Kakarot
    pub fn max_priority_fee_per_gas(&self) -> U128 {
        MAX_PRIORITY_FEE_PER_GAS
    }

    pub fn network(&self) -> &Network {
        &self.network
    }

    pub fn kakarot_contract(&self) -> Arc<KakarotContract<P>> {
        self.kakarot_contract.clone()
    }

    /// Returns the Kakarot contract address.
    pub fn kakarot_address(&self) -> FieldElement {
        self.kakarot_contract.address
    }

    /// Returns the Kakarot proxy account class hash.
    pub fn proxy_account_class_hash(&self) -> FieldElement {
        self.kakarot_contract.proxy_account_class_hash
    }

    /// Returns a reference to the Starknet provider.
    pub fn starknet_provider(&self) -> Arc<P> {
        self.starknet_provider.clone()
    }

    /// Returns a reference to the starknet account used for deployment
    pub fn deployer_account(&self) -> &SingleOwnerAccount<Arc<P>, LocalWallet> {
        &self.deployer_account
    }

    /// Returns the Starknet block number for a given block id.
    pub async fn map_block_id_to_block_number(&self, block_id: &StarknetBlockId) -> Result<u64, EthApiError<P::Error>> {
        match block_id {
            StarknetBlockId::Number(n) => Ok(*n),
            StarknetBlockId::Tag(_) => Ok(self.starknet_provider.block_number().await?),
            StarknetBlockId::Hash(_) => {
                let block = self.starknet_provider.get_block_with_tx_hashes(block_id).await?;
                match block {
                    MaybePendingBlockWithTxHashes::Block(block_with_tx_hashes) => Ok(block_with_tx_hashes.block_number),
                    _ => Err(ProviderError::StarknetError(StarknetErrorWithMessage {
                        code: MaybeUnknownErrorCode::Known(StarknetError::BlockNotFound),
                        message: "".to_string(),
                    })
                    .into()),
                }
            }
        }
    }

    /// Returns the EVM address associated with a given Starknet address for a given block id
    /// by calling the `get_evm_address` function on the Kakarot contract.
    pub async fn get_evm_address(
        &self,
        starknet_address: &FieldElement,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<Address, EthApiError<P::Error>> {
        let kakarot_account = KakarotAccount::new(*starknet_address, self.starknet_provider());
        kakarot_account.get_evm_address(starknet_block_id).await
    }

    /// Submits a Kakarot transaction to the Starknet provider.
    pub async fn submit_starknet_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
    ) -> Result<H256, EthApiError<P::Error>> {
        let transaction_result = self.starknet_provider.add_invoke_transaction(&request).await?;
        let waiter =
            TransactionWaiter::new(self.starknet_provider(), transaction_result.transaction_hash, 1000, 15_000);
        waiter.poll().await?;

        Ok(H256::from(transaction_result.transaction_hash.to_bytes_be()))
    }

    /// Returns the EVM address associated with a given Starknet address for a given block id
    /// by calling the `compute_starknet_address` function on the Kakarot contract.
    pub async fn compute_starknet_address(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<FieldElement, EthApiError<P::Error>> {
        let ethereum_address: Felt252Wrapper = ethereum_address.into();
        let ethereum_address = ethereum_address.into();

        self.kakarot_contract.compute_starknet_address(&ethereum_address, starknet_block_id).await
    }

    /// Returns the Ethereum transactions executed by the Kakarot contract by filtering the provided
    /// Starknet transaction.
    pub async fn filter_starknet_into_eth_txs(
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
    pub async fn get_eth_block_from_starknet_block(
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

    /// Get the simulation of the BroadcastedInvokeTransactionV1 result
    /// FIXME 306: make simulate_transaction agnostic of the provider (rn only works for
    /// a SequencerGatewayProvider on testnets and mainnet)
    pub async fn simulate_transaction(
        &self,
        request: BroadcastedInvokeTransaction,
        block_number: u64,
        skip_validate: bool,
    ) -> Result<TransactionSimulationInfo, EthApiError<P::Error>> {
        let client = Client::new();

        // build the url for simulate transaction
        let url = self.network.gateway_url();

        // if the url is invalid, return an empty simulation (allows to call simulate_transaction on Kakana,
        // Madara, etc.)
        if url.is_err() {
            let gas_usage = (*ESTIMATE_GAS).try_into().map_err(ConversionError::UintConversionError)?;
            let gas_price: Felt252Wrapper = (*MAX_FEE).into();
            let overall_fee = Felt252Wrapper::from(gas_usage) * gas_price.clone();
            return Ok(TransactionSimulationInfo {
                trace: TransactionTrace {
                    function_invocation: None,
                    fee_transfer_invocation: None,
                    validate_invocation: None,
                    signature: vec![],
                },
                fee_estimation: FeeEstimate {
                    gas_usage,
                    gas_price: gas_price.try_into()?,
                    overall_fee: overall_fee.try_into()?,
                    unit: FeeUnit::Wei,
                },
            });
        }

        let mut url = url
            .unwrap() // safe unwrap because we checked for error above
            .join("simulate_transaction")
            .map_err(|e| EthApiError::FeederGatewayError(format!("gateway url parsing error: {:?}", e)))?;

        // add the block number and skipValidate query params
        url.query_pairs_mut()
            .append_pair("blockNumber", &block_number.to_string())
            .append_pair("skipValidate", &skip_validate.to_string());

        // serialize the request
        let mut request = serde_json::to_value(request)
            .map_err(|e| EthApiError::FeederGatewayError(format!("request serializing error: {:?}", e)))?;
        // BroadcastedInvokeTransactionV1 gets serialized with type="INVOKE" but the simulate endpoint takes
        // type="INVOKE_FUNCTION"
        request["type"] = "INVOKE_FUNCTION".into();

        // post to the gateway
        let response = client
            .post(url)
            .json(&request)
            .send()
            .await
            .map_err(|e| EthApiError::FeederGatewayError(format!("gateway post error: {:?}", e)))?;

        // decode the response to a `TransactionSimulationInfo`
        let resp: TransactionSimulationInfo = response
            .error_for_status()
            .map_err(|e| EthApiError::FeederGatewayError(format!("http error: {:?}", e)))?
            .json()
            .await
            .map_err(|e| {
                EthApiError::FeederGatewayError(format!(
                    "error while decoding response body to TransactionSimulationInfo: {:?}",
                    e
                ))
            })?;

        Ok(resp)
    }

    pub async fn filter_events(&self, filter: EventFilterWithPage) -> Result<Vec<EmittedEvent>, EthApiError<P::Error>> {
        let provider = self.starknet_provider();

        let chunk_size = filter.result_page_request.chunk_size;
        let continuation_token = filter.result_page_request.continuation_token;
        let filter = filter.event_filter;

        let mut result = EventsPage { events: Vec::new(), continuation_token };
        let mut events = vec![];

        loop {
            result = provider.get_events(filter.clone(), result.continuation_token, chunk_size).await?;
            events.append(&mut result.events);

            if result.continuation_token.is_none() {
                break;
            }
        }

        Ok(events)
    }

    pub async fn check_eoa_account_exists(
        &self,
        ethereum_address: Address,
        starknet_block_id: &StarknetBlockId,
    ) -> Result<bool, EthApiError<P::Error>> {
        let eoa_account_starknet_address = self.compute_starknet_address(ethereum_address, starknet_block_id).await?;

        let result = self.get_evm_address(&eoa_account_starknet_address, starknet_block_id).await;

        let result: Result<bool, EthApiError<<P as Provider>::Error>> = match result {
            Ok(_) => Ok(true),
            Err(error) => match error {
                EthApiError::RequestError(error) => match error {
                    ProviderError::StarknetError(error) => match error {
                        StarknetErrorWithMessage {
                            code: MaybeUnknownErrorCode::Known(StarknetError::ContractNotFound),
                            ..
                        } => Ok(false),
                        _ => Err(EthApiError::from(ProviderError::StarknetError(error))),
                    },
                    _ => Err(EthApiError::from(error)),
                },
                _ => Err(error),
            },
        };

        result
    }

    pub async fn deploy_eoa(&self, ethereum_address: Address) -> Result<FieldElement, EthApiError<P::Error>> {
        let ethereum_address: FieldElement = Felt252Wrapper::from(ethereum_address).into();
        self.kakarot_contract.deploy_externally_owned_account(ethereum_address, &self.deployer_account).await
    }

    /// Given a transaction hash, waits for it to be confirmed on L2
    pub async fn wait_for_confirmation_on_l2(
        &self,
        transaction_hash: FieldElement,
    ) -> Result<(), EthApiError<P::Error>> {
        let waiter = TransactionWaiter::new(self.starknet_provider(), transaction_hash, 1000, 15_000);
        waiter.poll().await?;
        Ok(())
    }
}
