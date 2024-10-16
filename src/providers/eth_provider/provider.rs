use super::{
    constant::CALL_REQUEST_GAS_LIMIT,
    database::{ethereum::EthereumBlockStore, Database},
    error::{EthApiError, EthereumDataFormatError, EvmError, ExecutionError, TransactionError},
    starknet::kakarot_core::{
        self,
        core::{CallInput, KakarotCoreReader, Uint256},
        KAKAROT_ADDRESS,
    },
};
use crate::{
    constants::{ETH_CHAIN_ID, RELAYERS},
    into_via_try_wrapper, into_via_wrapper,
    models::block::{EthBlockId, EthBlockNumberOrTag},
    providers::{
        eth_provider::{
            error::CairoError,
            starknet::kakarot_core::core::{EthSendRawUnsignedTxOutput, KakarotCore},
            BlockProvider, GasProvider, LogProvider, ReceiptProvider, StateProvider, TransactionProvider,
        },
        sn_provider::StarknetProvider,
    },
};
use alloy_consensus::{SignableTransaction, TxLegacy};
use alloy_primitives::{TxKind, U256};
use alloy_rpc_types::{BlockHashOrNumber, TransactionRequest};
use cainome::cairo_serde::{CairoArrayLegacy, CairoSerde};
use eyre::Result;
use itertools::Itertools;
use mongodb::bson::doc;
use num_traits::cast::ToPrimitive;
use reth_primitives::{BlockId, BlockNumberOrTag};
use starknet::{
    accounts::{ExecutionEncoding, SingleOwnerAccount},
    core::types::{
        ExecuteInvocation, Felt, FunctionInvocation, InvokeTransactionTrace, RevertedInvocation, TransactionTrace,
    },
    signers::{LocalWallet, SigningKey},
};
use std::sync::Arc;
use tracing::{instrument, Instrument};
#[cfg(feature = "hive")]
use {
    crate::providers::eth_provider::error::SignatureError,
    crate::providers::eth_provider::starknet::kakarot_core::{
        account_contract::AccountContractReader, starknet_address,
    },
    crate::providers::eth_provider::utils::contract_not_found,
    alloy_primitives::Address,
};

/// A type alias representing a result type for Ethereum API operations.
///
/// This alias is used to simplify function signatures that return a `Result`
/// with an [`EthApiError`] as the error type.
pub type EthApiResult<T> = Result<T, EthApiError>;

/// A trait that defines the interface for an Ethereum Provider.
pub trait EthereumProvider:
    GasProvider + StateProvider + TransactionProvider + ReceiptProvider + LogProvider + BlockProvider
{
}

impl<T> EthereumProvider for T where
    T: GasProvider + StateProvider + TransactionProvider + ReceiptProvider + LogProvider + BlockProvider
{
}

/// Structure that implements the `EthereumProvider` trait.
/// Uses access to a database for certain data, while
/// the rest is fetched from the Starknet Provider.
#[derive(Debug, Clone)]
pub struct EthDataProvider<SP> {
    database: Database,
    starknet_provider: StarknetProvider<Arc<SP>>,
    pub chain_id: u64,
}

impl<SP> EthDataProvider<SP> {
    /// Returns a reference to the database.
    pub const fn database(&self) -> &Database {
        &self.database
    }

    /// Returns a reference to the Starknet provider.
    pub const fn starknet_provider(&self) -> &StarknetProvider<Arc<SP>> {
        &self.starknet_provider
    }

    /// Returns a reference to the underlying SP provider.
    pub fn starknet_provider_inner(&self) -> Arc<SP> {
        Arc::clone(&*self.starknet_provider)
    }
}

impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    pub fn new(database: Database, starknet_provider: SP) -> Self {
        let starknet_provider = StarknetProvider::new(Arc::new(starknet_provider));
        Self { database, starknet_provider, chain_id: *ETH_CHAIN_ID }
    }

    /// Prepare the call input for an estimate gas or call from a transaction request.
    #[instrument(skip(self, request), name = "prepare_call")]
    async fn prepare_call_input(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthApiResult<CallInput> {
        // unwrap option
        let to: kakarot_core::core::Option = {
            match request.to {
                Some(TxKind::Call(to)) => {
                    kakarot_core::core::Option { is_some: Felt::ONE, value: into_via_wrapper!(to) }
                }
                _ => kakarot_core::core::Option { is_some: Felt::ZERO, value: Felt::ZERO },
            }
        };

        // Here we check if CallRequest.origin is None, if so, we insert origin = address(0)
        let from = into_via_wrapper!(request.from.unwrap_or_default());

        let data = request.input.into_input().unwrap_or_default();
        let calldata: Vec<Felt> = data.into_iter().map_into().collect();

        let gas_limit = into_via_try_wrapper!(request.gas.unwrap_or(CALL_REQUEST_GAS_LIMIT))?;

        // We cannot unwrap_or_default() here because Kakarot.eth_call will
        // Reject transactions with gas_price < Kakarot.base_fee
        let gas_price = {
            let gas_price = match request.gas_price {
                Some(gas_price) => U256::from(gas_price),
                None => self.gas_price().await?,
            };
            into_via_try_wrapper!(gas_price)?
        };

        let value = Uint256 { low: into_via_try_wrapper!(request.value.unwrap_or_default())?, high: Felt::ZERO };

        // TODO: replace this by into_via_wrapper!(request.nonce.unwrap_or_default())
        //  when we can simulate the transaction instead of calling `eth_call`
        let nonce = {
            match request.nonce {
                Some(nonce) => into_via_wrapper!(nonce),
                None => match request.from {
                    None => Felt::ZERO,
                    Some(address) => into_via_try_wrapper!(self.transaction_count(address, block_id).await?)?,
                },
            }
        };

        Ok(CallInput { nonce, from, to, gas_limit, gas_price, value, calldata })
    }

    /// Call the Kakarot contract with the given request.
    pub(crate) async fn call_helper(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthApiResult<CairoArrayLegacy<Felt>> {
        tracing::trace!(?request);

        let starknet_block_id = self.to_starknet_block_id(block_id).await?;
        let call_input = self.prepare_call_input(request, block_id).await?;

        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, self.starknet_provider_inner());
        let span = tracing::span!(tracing::Level::INFO, "sn::eth_call");
        let call_output = kakarot_contract
            .eth_call(
                &call_input.nonce,
                &call_input.from,
                &call_input.to,
                &call_input.gas_limit,
                &call_input.gas_price,
                &call_input.value,
                &call_input.calldata.len().into(),
                &CairoArrayLegacy(call_input.calldata),
                &Felt::ZERO,
                &CairoArrayLegacy(vec![]),
            )
            .block_id(starknet_block_id)
            .call()
            .instrument(span)
            .await
            .map_err(ExecutionError::from)?;

        let return_data = call_output.return_data;
        if call_output.success == Felt::ZERO {
            return Err(ExecutionError::from(EvmError::from(return_data.0)).into());
        }
        Ok(return_data)
    }

    /// Estimate the gas used in Kakarot for the given request.
    pub(crate) async fn estimate_gas(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
    ) -> EthApiResult<u128> {
        // Serialize the unsigned transaction for simulation
        let gas_price = match request.gas_price {
            Some(gas_price) => gas_price,
            None => self.gas_price().await?.try_into().map_err(|_| EthereumDataFormatError::Primitive)?,
        };
        let data = request.input.into_input().unwrap_or_default();
        let tx = TxLegacy {
            chain_id: Some(*ETH_CHAIN_ID),
            nonce: request.nonce.unwrap_or_default(),
            gas_price,
            gas_limit: request.gas.unwrap_or_default(),
            to: request.to.unwrap_or_default(),
            value: request.value.unwrap_or_default(),
            input: data,
        };
        let ser_tx = CairoArrayLegacy(tx.encoded_for_signing().into_iter().map(Felt::from).collect::<Vec<_>>());
        let ser_tx_len = Felt::from(ser_tx.len());

        // Init a connected account to the first relayer in the set
        let block_id = self.to_starknet_block_id(block_id).await?;
        let wallet = LocalWallet::from_signing_key(SigningKey::from_random());
        let mut connected_account = SingleOwnerAccount::new(
            self.starknet_provider_inner(),
            wallet,
            RELAYERS.first().copied().expect("always at least one relayer"),
            self.chain_id.into(),
            ExecutionEncoding::New,
        );
        connected_account.set_block_id(block_id);

        let kakarot_contract = KakarotCore::new(*KAKAROT_ADDRESS, connected_account);
        let span = tracing::span!(tracing::Level::INFO, "sn::eth_send_raw_unsigned_tx");

        // Simulate the execution of an unsigned transaction
        let res = kakarot_contract
            .eth_send_raw_unsigned_tx(&ser_tx_len, &ser_tx)
            .simulate(true, true)
            .instrument(span)
            .await
            .map_err(|_| ExecutionError::CairoVm(CairoError::VmOutOfResources))?;

        let res = match res.transaction_trace {
            // Return the result if the transaction passes
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Success(FunctionInvocation { result, .. }),
                ..
            }) => result,
            // Return a cairo vm out of steps error in case we reverted with "out of steps"
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Reverted(RevertedInvocation { revert_reason }),
                ..
            }) if revert_reason.contains("RunResources has no remaining steps") => {
                return Err(ExecutionError::CairoVm(CairoError::VmOutOfResources).into());
            }
            // Return the error as normal otherwise.
            TransactionTrace::Invoke(InvokeTransactionTrace {
                execute_invocation: ExecuteInvocation::Reverted(RevertedInvocation { revert_reason }),
                ..
            }) => {
                return Err(TransactionError::Call(revert_reason.into()).into());
            }
            _ => unreachable!(),
        };
        let send_raw_tx_res =
            EthSendRawUnsignedTxOutput::cairo_deserialize(&res, 0).map_err(|err| TransactionError::Call(err.into()))?;

        let (return_data, gas, success) =
            (send_raw_tx_res.return_data, send_raw_tx_res.gas_used, send_raw_tx_res.success);

        if success == Felt::ZERO {
            return Err(ExecutionError::from(EvmError::from(return_data.0)).into());
        }
        let required_gas = gas.to_u128().ok_or(TransactionError::GasOverflow)?;
        Ok(required_gas)
    }

    /// Convert the given block id into a Starknet block id
    #[instrument(skip_all, ret)]
    pub async fn to_starknet_block_id(
        &self,
        block_id: impl Into<Option<BlockId>>,
    ) -> EthApiResult<starknet::core::types::BlockId> {
        match block_id.into() {
            Some(BlockId::Hash(hash)) => {
                Ok(EthBlockId::new(BlockId::Hash(hash)).try_into().map_err(EthereumDataFormatError::from)?)
            }
            Some(BlockId::Number(number_or_tag)) => {
                // There is a need to separate the BlockNumberOrTag case into three subcases
                // because pending Starknet blocks don't have a number.
                // 1. The block number corresponds to a Starknet pending block, then we return the pending tag
                // 2. The block number corresponds to a Starknet sealed block, then we return the block number
                // 3. The block number is not found, then we return an error
                match number_or_tag {
                    BlockNumberOrTag::Number(number) => {
                        let header = self
                            .database
                            .header(number.into())
                            .await?
                            .ok_or(EthApiError::UnknownBlockNumber(Some(number)))?;
                        // If the block hash is zero, then the block corresponds to a Starknet pending block
                        if header.hash.is_zero() {
                            Ok(starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending))
                        } else {
                            Ok(starknet::core::types::BlockId::Number(number))
                        }
                    }
                    _ => Ok(EthBlockNumberOrTag::from(number_or_tag).into()),
                }
            }
            None => Ok(starknet::core::types::BlockId::Tag(starknet::core::types::BlockTag::Pending)),
        }
    }

    /// Converts the given [`BlockNumberOrTag`] into a block number.
    #[instrument(skip(self))]
    pub(crate) async fn tag_into_block_number(&self, tag: BlockNumberOrTag) -> EthApiResult<u64> {
        match tag {
            // Converts the tag representing the earliest block into block number 0.
            BlockNumberOrTag::Earliest => Ok(0),
            // Converts the tag containing a specific block number into a `U64`.
            BlockNumberOrTag::Number(number) => Ok(number),
            // Returns `self.block_number()` which is the block number of the latest finalized block.
            BlockNumberOrTag::Latest | BlockNumberOrTag::Finalized | BlockNumberOrTag::Safe => {
                self.block_number().await.map(|x| x.to())
            }
            // Adds 1 to the block number of the latest finalized block.
            BlockNumberOrTag::Pending => Ok(self.block_number().await?.to::<u64>().saturating_add(1)),
        }
    }

    /// Converts the given [`BlockId`] into a [`BlockHashOrNumber`].
    #[instrument(skip_all, ret)]
    pub(crate) async fn block_id_into_block_number_or_hash(
        &self,
        block_id: BlockId,
    ) -> EthApiResult<BlockHashOrNumber> {
        match block_id {
            BlockId::Hash(hash) => Ok(BlockHashOrNumber::Hash(hash.into())),
            BlockId::Number(number_or_tag) => Ok(self.tag_into_block_number(number_or_tag).await?.into()),
        }
    }
}

#[cfg(feature = "hive")]
impl<SP> EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    /// Deploy the EVM transaction signer if a corresponding contract is not found on
    /// Starknet.
    pub(crate) async fn deploy_evm_transaction_signer(&self, signer: Address) -> EthApiResult<()> {
        use crate::providers::eth_provider::constant::hive::{DEPLOY_WALLET, DEPLOY_WALLET_NONCE};
        use starknet::{
            accounts::ExecutionV1,
            core::{
                types::{BlockTag, Call},
                utils::get_selector_from_name,
            },
        };

        let signer_starknet_address = starknet_address(signer);
        let account_contract = AccountContractReader::new(signer_starknet_address, self.starknet_provider_inner());
        let maybe_is_initialized = account_contract
            .is_initialized()
            .block_id(starknet::core::types::BlockId::Tag(BlockTag::Latest))
            .call()
            .await;

        if contract_not_found(&maybe_is_initialized) {
            let execution = ExecutionV1::new(
                vec![Call {
                    to: *KAKAROT_ADDRESS,
                    selector: get_selector_from_name("deploy_externally_owned_account").unwrap(),
                    calldata: vec![into_via_wrapper!(signer)],
                }],
                &*DEPLOY_WALLET,
            );

            let mut nonce = DEPLOY_WALLET_NONCE.lock().await;
            let current_nonce = *nonce;

            let prepared_execution = execution
                .nonce(current_nonce)
                .max_fee(u64::MAX.into())
                .prepared()
                .map_err(|_| EthApiError::EthereumDataFormat(EthereumDataFormatError::TransactionConversion))?;

            let _ = prepared_execution.send().await.map_err(|_| SignatureError::SigningFailure)?;

            *nonce += Felt::ONE;
            drop(nonce);
        };

        Ok(())
    }
}
