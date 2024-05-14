use std::fmt::LowerHex;

use super::constant::LOGS_TOPICS_HEX_STRING_LEN;
use cainome::cairo_serde::Error;
use mongodb::bson::{doc, Document};
use reth_primitives::{U128, U256};
use reth_rpc_types::{Topic, ValueOrArray};
use starknet::{
    core::types::{ContractErrorData, StarknetError},
    providers::ProviderError,
};

/// Converts an array of topics into a MongoDB filter.
pub(crate) fn to_logs_filter(topics: [Topic; 4]) -> Document {
    // Converts the topics to [Option<Vec<Topic>>;4]
    let topics = topics
        .into_iter()
        .map(|t| {
            t.to_value_or_array().map(|t| match t {
                ValueOrArray::Value(topic) => vec![topic],
                ValueOrArray::Array(topics) => topics,
            })
        })
        .collect::<Vec<_>>();

    // Count the amount of topics that are not None
    // If all topics are None, return a filter that checks if the log.topics field exists
    let topics_len = topics.iter().fold(0usize, |acc, t| acc + t.as_ref().map_or(0, |_| 1));
    if topics_len == 0 {
        return doc! { "log.topics": {"$exists": true} };
    }

    let mut filter = vec![];

    // Iterate over the topics and add the filter to the filter vector
    for (index, maybe_topic) in topics.iter().enumerate() {
        // If the topic is None, skip it.
        if let Some(t) = maybe_topic {
            let topics = t.iter().map(|t| format_hex(t, LOGS_TOPICS_HEX_STRING_LEN)).collect::<Vec<_>>();
            let key = format!("log.topics.{}", index);
            // If the topic array has only one element, use an equality filter
            if topics.len() == 1 {
                filter.push(doc! {key: topics[0].clone()});
            } else {
                // If the topic array has more than one element, use an $in filter
                filter.push(doc! {key: {"$in": topics}});
            }
        }
    }

    doc! {"$and": filter}
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
    use proptest::prelude::*;
    use reth_primitives::B256;
    use reth_rpc_types::FilterSet;
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

    #[test]
    fn test_split_u256() {
        // Define a property-based test using Proptest
        proptest!(|(value in any::<U256>())| {
            // Call the split_u256 function to split the U256 value into two u128 values
            let result = split_u256::<u128>(value);

            // Combine the two u128 values into a hexadecimal string
            let combined_hex = format!("{:#x}{:0width$x}", result[1], result[0], width = 32);

            // Assertion to check the equality with the original U256 value
            assert_eq!(U256::from_str(&combined_hex).unwrap(), value);
        });
    }

    #[test]
    fn test_log_filter_empty() {
        // Given
        let topics = [Topic::default(), Topic::default(), Topic::default(), Topic::default()];

        // When
        let filter = to_logs_filter(topics);

        // Then
        assert_eq!(filter, doc! { "log.topics": {"$exists": true} });
    }

    #[test]
    fn test_log_filter() {
        // Given
        let topics: [FilterSet<B256>; 4] = [
            vec![B256::left_padding_from(&[1]), B256::left_padding_from(&[2])].into(),
            B256::left_padding_from(&[3]).into(),
            B256::left_padding_from(&[4]).into(),
            vec![B256::left_padding_from(&[5]), B256::left_padding_from(&[6])].into(),
        ];

        // When
        let filter = to_logs_filter(topics);

        // Then
        assert_eq!(
            filter,
            doc! { "$and": [
                    { "log.topics.0": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000000000000000000000000000002"]} },
                    { "log.topics.1": "0x0000000000000000000000000000000000000000000000000000000000000003" },
                    { "log.topics.2": "0x0000000000000000000000000000000000000000000000000000000000000004" },
                    { "log.topics.3": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000005", "0x0000000000000000000000000000000000000000000000000000000000000006"]} },
                ]
            }
        );
    }
}
