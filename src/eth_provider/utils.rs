use std::fmt::LowerHex;

use mongodb::bson::{doc, Document};
use reth_primitives::{AccessList, Transaction, TransactionKind, TxEip1559};
use reth_primitives::{U128, U256, U64};
use reth_rpc_types::CallRequest;

use crate::models::errors::ConversionError;

use super::constant::{BASE_FEE_PER_GAS, MAX_PRIORITY_FEE_PER_GAS};
use super::provider::EthProviderResult;

/// Converts an iterator of `Into<D>` into a `Vec<D>`.
pub(crate) fn iter_into<D, S: Into<D>>(iter: impl IntoIterator<Item = S>) -> Vec<D> {
    iter.into_iter().map(Into::into).collect::<Vec<_>>()
}

/// Converts an iterator of `TryInto<u8>` into a `FromIterator<u8>`.
pub(crate) fn try_from_u8_iterator<I: TryInto<u8>, T: FromIterator<u8>>(it: impl Iterator<Item = I>) -> T {
    it.filter_map(|x| TryInto::<u8>::try_into(x).ok()).collect()
}

pub(crate) fn format_hex(value: impl LowerHex, width: usize) -> String {
    // Add 2 to the width to account for the 0x prefix.
    let s = format!("{:#0width$x}", value, width = width + 2);
    // This can happen because of the LowerHex implementation for Uint,
    // which just formats 0 into 0x0, ignoring the width.
    if s.len() < width {
        return format!("0x{:0>width$}", &s[2..], width = width);
    }
    s
}

/// Converts a key and value into a MongoDB filter.
pub(crate) fn into_filter<T>(key: &str, value: T, width: usize) -> Document
where
    T: LowerHex,
{
    doc! {key: format_hex(value, width)}
}

/// Helper function to split a U256 value into two generic values
/// implementing the From<u128> trait
pub fn split_u256<T: From<u128>>(value: U256) -> [T; 2] {
    let low: u128 = (value & U256::from(U128::MAX)).try_into().unwrap(); // safe to unwrap
    let high: U256 = value >> 128;
    let high: u128 = high.try_into().unwrap(); // safe to unwrap
    [T::from(low), T::from(high)]
}

pub fn call_to_transaction(call: CallRequest, chain_id: U64, nonce: U64) -> EthProviderResult<Transaction> {
    let chain_id = call.chain_id.unwrap_or(chain_id).as_u64();

    let gas_limit = call.gas.unwrap_or_default().try_into().map_err(ConversionError::from)?;
    let max_fee_per_gas = call
        .max_fee_per_gas
        .unwrap_or_else(|| U256::from(*BASE_FEE_PER_GAS))
        .try_into()
        .map_err(ConversionError::from)?;
    let max_priority_fee_per_gas = call
        .max_priority_fee_per_gas
        .unwrap_or_else(|| U256::from(*MAX_PRIORITY_FEE_PER_GAS))
        .try_into()
        .map_err(ConversionError::from)?;

    let to = call.to.map_or(TransactionKind::Create, TransactionKind::Call);
    let value = call.value.unwrap_or_default().try_into().map_err(ConversionError::from)?;
    let data = call.input.unique_input().unwrap_or_default().cloned().unwrap_or_default();

    Ok(Transaction::Eip1559(TxEip1559 {
        chain_id,
        nonce: nonce.as_u64(),
        gas_limit,
        max_fee_per_gas,
        max_priority_fee_per_gas,
        to,
        value,
        access_list: AccessList(vec![]),
        input: data,
    }))
}

pub(crate) fn contract_not_found<T>(err: &Result<T, impl std::error::Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => err.to_string().contains("Contract not found"),
    }
}
