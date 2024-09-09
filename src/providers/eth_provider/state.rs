use super::{
    database::state::{EthCacheDatabase, EthDatabase},
    error::{EthApiError, ExecutionError, TransactionError},
    starknet::kakarot_core::{account_contract::AccountContractReader, starknet_address},
    utils::{contract_not_found, entrypoint_not_found, split_u256},
};
use crate::{
    into_via_wrapper,
    models::felt::Felt252Wrapper,
    providers::eth_provider::{
        provider::{EthApiResult, EthDataProvider},
        BlockProvider, ChainProvider,
    },
};
use async_trait::async_trait;
use auto_impl::auto_impl;
use mongodb::bson::doc;
use num_traits::cast::ToPrimitive;
use reth_evm_ethereum::EthEvmConfig;
use reth_node_api::ConfigureEvm;
use reth_primitives::{Address, BlockId, Bytes, B256, U256};
use reth_revm::{
    db::CacheDB,
    primitives::{BlockEnv, CfgEnv, CfgEnvWithHandlerCfg, HandlerCfg, SpecId},
};
use reth_rpc_eth_types::{error::ensure_success, revm_utils::prepare_call_env};
use reth_rpc_types::{
    serde_helpers::JsonStorageKey,
    state::{EvmOverrides, StateOverride},
    BlockOverrides, Header, TransactionRequest,
};
use starknet::core::utils::get_storage_var_address;
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait StateProvider: ChainProvider + BlockProvider {
    /// Returns the balance of an address in native eth.
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256>;

    /// Returns the storage of an address at a certain index.
    async fn storage_at(
        &self,
        address: Address,
        index: JsonStorageKey,
        block_id: Option<BlockId>,
    ) -> EthApiResult<B256>;

    /// Returns the code for the address at the given block.
    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<Bytes>;

    /// Returns the result of a call.
    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> EthApiResult<Bytes>;
}

#[async_trait]
impl<SP> StateProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn balance(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<U256> {
        // Convert the optional Ethereum block ID to a Starknet block ID.
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;
        // Get the balance of the address at the given block ID.
        self.starknet_provider().balance_at(starknet_address(address), starknet_block_id).await.map_err(Into::into)
    }

    async fn storage_at(
        &self,
        address: Address,
        index: JsonStorageKey,
        block_id: Option<BlockId>,
    ) -> EthApiResult<B256> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let contract = AccountContractReader::new(address, self.starknet_provider_inner());

        let keys = split_u256(index.0);
        let storage_address = get_storage_var_address("Account_storage", &keys).expect("Storage var name is not ASCII");

        let span = tracing::span!(tracing::Level::INFO, "sn::storage");
        let maybe_storage =
            contract.storage(&storage_address).block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&maybe_storage) || entrypoint_not_found(&maybe_storage) {
            return Ok(U256::ZERO.into());
        }

        let storage = maybe_storage.map_err(ExecutionError::from)?.value;
        let low: U256 = into_via_wrapper!(storage.low);
        let high: U256 = into_via_wrapper!(storage.high);
        let storage: U256 = low + (high << 128);

        Ok(storage.into())
    }

    async fn get_code(&self, address: Address, block_id: Option<BlockId>) -> EthApiResult<Bytes> {
        let starknet_block_id = self.to_starknet_block_id(block_id).await?;

        let address = starknet_address(address);
        let account_contract = AccountContractReader::new(address, self.starknet_provider_inner());
        let span = tracing::span!(tracing::Level::INFO, "sn::code");
        let bytecode = account_contract.bytecode().block_id(starknet_block_id).call().instrument(span).await;

        if contract_not_found(&bytecode) || entrypoint_not_found(&bytecode) {
            return Ok(Bytes::default());
        }

        let bytecode = bytecode.map_err(ExecutionError::from)?.bytecode.0;

        Ok(Bytes::from(bytecode.into_iter().filter_map(|x| x.to_u8()).collect::<Vec<_>>()))
    }

    async fn call(
        &self,
        request: TransactionRequest,
        block_id: Option<BlockId>,
        state_overrides: Option<StateOverride>,
        block_overrides: Option<Box<BlockOverrides>>,
    ) -> EthApiResult<Bytes> {
        // Create the EVM overrides from the state and block overrides.
        let evm_overrides = EvmOverrides::new(state_overrides, block_overrides);

        // Check if either state_overrides or block_overrides is present.
        if evm_overrides.has_state() || evm_overrides.has_block() {
            // Create the configuration environment with the chain ID.
            let cfg_env = CfgEnv::default().with_chain_id(self.chain_id().await?.unwrap_or_default().to());

            // Retrieve the block header details.
            let Header { number, timestamp, miner, base_fee_per_gas, difficulty, .. } =
                self.header(&block_id.unwrap_or_default()).await?.unwrap_or_default();

            // Create the block environment with the retrieved header details and transaction request.
            let block_env = BlockEnv {
                number: U256::from(number.unwrap_or_default()),
                timestamp: U256::from(timestamp),
                gas_limit: U256::from(request.gas.unwrap_or_default()),
                coinbase: miner,
                basefee: U256::from(base_fee_per_gas.unwrap_or_default()),
                prevrandao: Some(B256::from_slice(&difficulty.to_be_bytes::<32>()[..])),
                ..Default::default()
            };

            // Combine the configuration environment with the handler configuration.
            let cfg_env_with_handler_cfg =
                CfgEnvWithHandlerCfg { cfg_env, handler_cfg: HandlerCfg::new(SpecId::CANCUN) };

            // Create a snapshot of the Ethereum database using the block ID.
            let mut db = EthCacheDatabase(CacheDB::new(EthDatabase::new(self, block_id.unwrap_or_default())));

            // Prepare the call environment with the transaction request, gas limit, and overrides.
            let env = prepare_call_env(
                cfg_env_with_handler_cfg,
                block_env,
                request.clone(),
                request.gas.unwrap_or_default().try_into().expect("Gas limit is too large"),
                &mut db.0,
                evm_overrides,
            )?;

            // Execute the transaction using the configured EVM asynchronously.
            let res = EthEvmConfig::default()
                .evm_with_env(db.0, env)
                .transact()
                .map_err(|err| <TransactionError as Into<EthApiError>>::into(TransactionError::Call(err.into())))?;

            // Ensure the transaction was successful and return the result.
            return Ok(ensure_success(res.result)?);
        }

        // If no state or block overrides are present, call the helper function to execute the call.
        let output = self.call_helper(request, block_id).await?;
        Ok(Bytes::from(output.0.into_iter().filter_map(|x| x.to_u8()).collect::<Vec<_>>()))
    }
}
