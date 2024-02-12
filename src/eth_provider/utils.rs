use std::fmt::LowerHex;

use cainome::cairo_serde::Error;
use mongodb::bson::{doc, Document};
use reth_primitives::{U128, U256};
use starknet::{core::types::StarknetError, providers::ProviderError};

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
    // `s.len() < width` can happen because of the LowerHex implementation
    // for Uint, which just formats 0 into 0x0, ignoring the width.
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

pub(crate) const fn contract_not_found<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => matches!(err, Error::Provider(ProviderError::StarknetError(StarknetError::ContractNotFound))),
    }
}
