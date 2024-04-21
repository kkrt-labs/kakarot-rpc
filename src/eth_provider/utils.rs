use std::fmt::LowerHex;

use cainome::cairo_serde::Error;
use mongodb::bson::{doc, Document};
use reth_primitives::{U128, U256};
use starknet::{
    core::types::{ContractErrorData, StarknetError},
    providers::ProviderError,
};

/// Converts an iterator of `TryInto<u8>` into a `FromIterator<u8>`.
#[inline]
pub(crate) fn try_from_u8_iterator<I: TryInto<u8>, T: FromIterator<u8>>(it: impl IntoIterator<Item = I>) -> T {
    it.into_iter().filter_map(|x| TryInto::<u8>::try_into(x).ok()).collect()
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
pub fn into_filter<T>(key: &str, value: &T, width: usize) -> Document
where
    T: LowerHex,
{
    doc! {key: format_hex(value, width)}
}

/// Splits a U256 value into two generic values implementing the From<u128> trait
#[inline]
pub fn split_u256<T: From<u128>>(value: impl Into<U256>) -> [T; 2] {
    let value: U256 = value.into();
    let low: u128 = (value & U256::from(U128::MAX)).try_into().unwrap(); // safe to unwrap
    let high: U256 = value >> 128;
    let high: u128 = high.try_into().unwrap(); // safe to unwrap
    [T::from(low), T::from(high)]
}

/// Checks if the error is a contract not found error.
#[inline]
pub(crate) const fn contract_not_found<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => matches!(err, Error::Provider(ProviderError::StarknetError(StarknetError::ContractNotFound))),
    }
}

/// Checks if the error is an entrypoint not found error.
#[inline]
pub(crate) fn entrypoint_not_found<T>(err: &Result<T, Error>) -> bool {
    match err {
        Ok(_) => false,
        Err(err) => matches!(
            err,
            Error::Provider(ProviderError::StarknetError(StarknetError::ContractError(ContractErrorData {
                revert_error: reason
            }))) if reason.contains("Entry point") && reason.contains("not found in contract")
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reth_primitives::B256;
    use std::str::FromStr;

    #[test]
    fn test_into_filter_with_padding() {
        assert_eq!(into_filter::<u64>("test_key", &0x1234, 10), doc! {"test_key": "0x0000001234"});
        assert_eq!(
            into_filter::<B256>(
                "test_key",
                &B256::from_str("0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3").unwrap(),
                64
            ),
            doc! {"test_key": "0xd4e56740f876aef8c010b86a40d5f56745a118d0906a34e69aec8c0db1cb8fa3"}
        );
        assert_eq!(
            into_filter::<B256>("test_key", &B256::default(), 64),
            doc! {"test_key": format!("0x{}", "0".repeat(64))}
        );
    }
}
