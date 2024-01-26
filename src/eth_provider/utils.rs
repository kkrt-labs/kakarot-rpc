use std::fmt::LowerHex;

use mongodb::bson::{doc, Document};
use reth_primitives::{U128, U256};

/// Converts an iterator of `Into<D>` into a `Vec<D>`.
pub(crate) fn iter_into<D, S: Into<D>>(iter: impl IntoIterator<Item = S>) -> Vec<D> {
    iter.into_iter().map(Into::into).collect::<Vec<_>>()
}

/// Converts a key and value into a MongoDB filter.
pub(crate) fn into_filter<T>(key: &str, value: T, width: usize) -> Document
where
    T: LowerHex,
{
    doc! {key: format!("0x{:0width$x}", value, width = width)}
}

/// Helper function to split a U256 value into two generic values
/// implementing the From<u128> trait
pub fn split_u256<T: From<u128>>(value: U256) -> [T; 2] {
    let low: u128 = (value & U256::from(U128::MAX)).try_into().unwrap(); // safe to unwrap
    let high: U256 = value >> 128;
    let high: u128 = high.try_into().unwrap(); // safe to unwrap
    [T::from(low), T::from(high)]
}
