use std::str;
use std::sync::Arc;

use jsonrpsee::core::{async_trait, RpcResult as Result};
use kakarot_rpc_core::client::api::KakarotEthApi;
use kakarot_rpc_core::client::errors::EthApiError;
use kakarot_rpc_core::client::helpers::vec_felt_to_bytes;
use kakarot_rpc_core::models::allowance::TokenAllowance;
use kakarot_rpc_core::models::balance::TokenBalances;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_rpc_core::models::metadata::TokenMetadata;
use kakarot_rpc_core::models::ConversionError;
use reth_primitives::{keccak256, Address, BlockId, BlockNumberOrTag, Bytes, U256};
use starknet::core::types::FieldElement;
use starknet::providers::Provider;

use crate::api::alchemy_api::AlchemyApiServer;

/// The RPC module for the Ethereum protocol required by Kakarot.
pub struct AlchemyRpc<P: Provider + Send + Sync> {
    pub kakarot_client: Arc<dyn KakarotEthApi<P>>,
}

impl<P: Provider + Send + Sync> AlchemyRpc<P> {
    #[must_use]
    pub fn new(kakarot_client: Arc<dyn KakarotEthApi<P>>) -> Self {
        Self { kakarot_client }
    }
}

#[async_trait]
impl<P: Provider + Send + Sync + 'static> AlchemyApiServer for AlchemyRpc<P> {
    async fn token_allowance(
        &self,
        contract_address: Address,
        account_address: Address,
        spender_address: Address,
    ) -> Result<TokenAllowance> {
        let entry_point: Felt252Wrapper = keccak256("allowance(account, spender)")
            .try_into()
            .map_err(|e: ConversionError<()>| EthApiError::<P::Error>::ConversionError(e.to_string()))?;
        let entry_point: FieldElement = entry_point.into();

        let account_addr: Felt252Wrapper = account_address.into();
        let account_addr: FieldElement = account_addr.into();

        let spender_addr: Felt252Wrapper = spender_address.into();
        let spender_addr: FieldElement = spender_addr.into();

        let calldata = vec![entry_point, account_addr, spender_addr];

        let handle = self
            .kakarot_client
            .call(contract_address, Bytes::from(vec_felt_to_bytes(calldata).0), BlockId::from(BlockNumberOrTag::Latest))
            .await;

        let token_allowance = match handle {
            Ok(call) => {
                let allowance = U256::try_from_be_slice(call.as_ref())
                    .ok_or(ConversionError::<()>::Other("error converting from Bytes to U256".into()))
                    .unwrap();
                TokenAllowance { result: Some(allowance), error: None }
            }
            Err(e) => TokenAllowance { result: None, error: Some(format!("kakarot_getTokenAllowance Error: {e}")) },
        };

        Ok(token_allowance)
    }

    async fn token_balances(&self, address: Address, contract_addresses: Vec<Address>) -> Result<TokenBalances> {
        let token_balances = self.kakarot_client.token_balances(address, contract_addresses).await?;
        Ok(token_balances)
    }

    async fn token_metadata(&self, contract_address: Address) -> Result<TokenMetadata> {
        let decimals_entry_point: Felt252Wrapper = keccak256("decimals()")
            .try_into()
            .map_err(|e: ConversionError<()>| EthApiError::<P::Error>::ConversionError(e.to_string()))?;
        let decimals_entry_point: FieldElement = decimals_entry_point.into();

        let name_entry_point: Felt252Wrapper = keccak256("name()")
            .try_into()
            .map_err(|e: ConversionError<()>| EthApiError::<P::Error>::ConversionError(e.to_string()))?;
        let name_entry_point: FieldElement = name_entry_point.into();

        let symbol_entry_point: Felt252Wrapper = keccak256("symbol()")
            .try_into()
            .map_err(|e: ConversionError<()>| EthApiError::<P::Error>::ConversionError(e.to_string()))?;
        let symbol_entry_point: FieldElement = symbol_entry_point.into();

        let decimals_calldata = vec![decimals_entry_point];
        let name_calldata = vec![name_entry_point];
        let symbol_calldata = vec![symbol_entry_point];

        let decimals_handle = self
            .kakarot_client
            .call(
                contract_address,
                Bytes::from(vec_felt_to_bytes(decimals_calldata).0),
                BlockId::from(BlockNumberOrTag::Latest),
            )
            .await;

        let name_handle = self
            .kakarot_client
            .call(
                contract_address,
                Bytes::from(vec_felt_to_bytes(name_calldata).0),
                BlockId::from(BlockNumberOrTag::Latest),
            )
            .await;

        let symbol_handle = self
            .kakarot_client
            .call(
                contract_address,
                Bytes::from(vec_felt_to_bytes(symbol_calldata).0),
                BlockId::from(BlockNumberOrTag::Latest),
            )
            .await;

        let token_metadata = match (decimals_handle, name_handle, symbol_handle) {
            (Ok(decimals_call), Ok(name_call), Ok(symbol_call)) => {
                let decimals = U256::try_from_be_slice(decimals_call.as_ref())
                    .ok_or(ConversionError::<()>::Other("error converting from Bytes to U256".into()))
                    .unwrap();
                let name = str::from_utf8(name_call.as_ref()).unwrap();
                let symbol = str::from_utf8(symbol_call.as_ref()).unwrap();
                TokenMetadata {
                    decimals: Some(decimals),
                    name: Some(name.to_string()),
                    symbol: Some(symbol.to_string()),
                    error: None,
                }
            }
            (Err(e), _, _) | (_, Err(e), _) | (_, _, Err(e)) => TokenMetadata {
                decimals: None,
                name: None,
                symbol: None,
                error: Some(format!("kakarot_getTokenMetadata Error: {e}")),
            },
        };

        Ok(token_metadata)
    }
}
