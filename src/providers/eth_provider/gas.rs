use super::{
    constant::BLOCK_NUMBER_HEX_STRING_LEN,
    error::{ExecutionError, KakarotError},
    starknet::kakarot_core::{core::KakarotCoreReader, KAKAROT_ADDRESS},
};
use crate::{
    into_via_wrapper,
    providers::eth_provider::{
        database::{filter::format_hex, types::header::StoredHeader},
        provider::{EthApiResult, EthDataProvider},
    },
};
use alloy_primitives::{U256, U64};
use alloy_rpc_types::{FeeHistory, TransactionRequest};
use async_trait::async_trait;
use auto_impl::auto_impl;
use eyre::eyre;
use mongodb::bson::doc;
use reth_primitives::{BlockId, BlockNumberOrTag};
use tracing::Instrument;

#[async_trait]
#[auto_impl(Arc, &)]
pub trait GasProvider {
    /// Returns the result of a estimate gas.
    async fn estimate_gas(&self, call: TransactionRequest, block_id: Option<BlockId>) -> EthApiResult<U256>;

    /// Returns the fee history given a block count and a newest block number.
    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        reward_percentiles: Option<Vec<f64>>,
    ) -> EthApiResult<FeeHistory>;

    /// Returns the current gas price.
    async fn gas_price(&self) -> EthApiResult<U256>;
}

#[async_trait]
impl<SP> GasProvider for EthDataProvider<SP>
where
    SP: starknet::providers::Provider + Send + Sync,
{
    async fn estimate_gas(&self, request: TransactionRequest, block_id: Option<BlockId>) -> EthApiResult<U256> {
        // Set a high gas limit to make sure the transaction will not fail due to gas.
        let request = TransactionRequest { gas: Some(u64::MAX), ..request };

        let gas_used = self.estimate_gas(request, block_id).await?;

        // Increase the gas used by 200% to make sure the transaction will not fail due to gas.
        // This is a temporary solution until we have a proper gas estimation.
        // Does not apply to Hive feature otherwise end2end tests will fail.
        let gas_used = if cfg!(feature = "hive") { gas_used } else { gas_used * 2 };
        Ok(U256::from(gas_used))
    }

    async fn fee_history(
        &self,
        block_count: U64,
        newest_block: BlockNumberOrTag,
        _reward_percentiles: Option<Vec<f64>>,
    ) -> EthApiResult<FeeHistory> {
        if block_count == U64::ZERO {
            return Ok(FeeHistory::default());
        }

        let end_block = self.tag_into_block_number(newest_block).await?;
        let end_block_plus_one = end_block.saturating_add(1);

        // 0 <= start_block <= end_block
        let start_block = end_block_plus_one.saturating_sub(block_count.to());

        let header_filter = doc! {"$and": [ { "header.number": { "$gte": format_hex(start_block, BLOCK_NUMBER_HEX_STRING_LEN) } }, { "header.number": { "$lte": format_hex(end_block, BLOCK_NUMBER_HEX_STRING_LEN) } } ] };
        let blocks: Vec<StoredHeader> = self.database().get(header_filter, None).await?;

        if blocks.is_empty() {
            return Err(
                KakarotError::from(mongodb::error::Error::custom(eyre!("No blocks found in the database"))).into()
            );
        }

        let gas_used_ratio = blocks
            .iter()
            .map(|header| {
                let gas_used = header.gas_used as f64;
                let mut gas_limit = header.gas_limit as f64;
                if gas_limit == 0. {
                    gas_limit = 1.;
                };
                gas_used / gas_limit
            })
            .collect();

        let mut base_fee_per_gas =
            blocks.iter().map(|header| header.base_fee_per_gas.unwrap_or_default()).collect::<Vec<_>>();
        // TODO(EIP1559): Remove this when proper base fee computation: if gas_ratio > 50%, increase base_fee_per_gas
        base_fee_per_gas.extend_from_within((base_fee_per_gas.len() - 1)..);

        Ok(FeeHistory {
            base_fee_per_gas: base_fee_per_gas.into_iter().map(Into::into).collect(),
            gas_used_ratio,
            oldest_block: start_block,
            reward: Some(vec![]),
            ..Default::default()
        })
    }

    async fn gas_price(&self) -> EthApiResult<U256> {
        let kakarot_contract = KakarotCoreReader::new(*KAKAROT_ADDRESS, self.starknet_provider_inner());
        let span = tracing::span!(tracing::Level::INFO, "sn::base_fee");
        let gas_price =
            kakarot_contract.get_base_fee().call().instrument(span).await.map_err(ExecutionError::from)?.base_fee;
        Ok(into_via_wrapper!(gas_price))
    }
}
