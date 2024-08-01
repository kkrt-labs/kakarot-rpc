use crate::providers::eth_provider::constant::{
    ADDRESS_HEX_STRING_LEN, BLOCK_NUMBER_HEX_STRING_LEN, HASH_HEX_STRING_LEN, LOGS_TOPICS_HEX_STRING_LEN,
    U64_HEX_STRING_LEN,
};
use mongodb::bson::{doc, Document};
use reth_primitives::{Address, B256};
use reth_rpc_types::{BlockHashOrNumber, Index, Topic};
use std::fmt::{Display, LowerHex};

/// A trait that defines possible key filters for blocks in the
/// Ethereum database.
pub trait BlockFiltering {
    /// Returns the key for the block hash.
    fn block_hash(&self) -> &'static str;
    /// Returns the key for the block number.
    fn block_number(&self) -> &'static str;
}

/// A trait that defines possible key filters for transactions in the
/// Ethereum database.
pub trait TransactionFiltering {
    /// Returns the key for the transaction hash.
    fn transaction_hash(&self) -> &'static str;
    /// Returns the key for the transaction index in the block.
    fn transaction_index(&self) -> &'static str;
}

/// A trait that defines possible key filters for logs in the
/// Ethereum database.
pub trait LogFiltering {
    /// Returns the key for the transaction hash.
    fn address(&self) -> &'static str;
}

/// A transaction type used as a target for the filter.
#[derive(Debug, Default)]
pub struct Transaction;

impl Display for Transaction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "tx")
    }
}

impl BlockFiltering for Transaction {
    fn block_hash(&self) -> &'static str {
        "blockHash"
    }

    fn block_number(&self) -> &'static str {
        "blockNumber"
    }
}

impl TransactionFiltering for Transaction {
    fn transaction_hash(&self) -> &'static str {
        "hash"
    }

    fn transaction_index(&self) -> &'static str {
        "transactionIndex"
    }
}

/// A receipt type used as a target for the filter.
#[derive(Debug, Default)]
pub struct Receipt;

impl Display for Receipt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "receipt")
    }
}

impl BlockFiltering for Receipt {
    fn block_hash(&self) -> &'static str {
        "blockHash"
    }

    fn block_number(&self) -> &'static str {
        "blockNumber"
    }
}

impl TransactionFiltering for Receipt {
    fn transaction_hash(&self) -> &'static str {
        "transactionHash"
    }

    fn transaction_index(&self) -> &'static str {
        "transactionIndex"
    }
}

/// A header type used as a target for the filter.
#[derive(Debug, Default)]
pub struct Header;

impl Display for Header {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "header")
    }
}

impl BlockFiltering for Header {
    fn block_hash(&self) -> &'static str {
        "hash"
    }

    fn block_number(&self) -> &'static str {
        "number"
    }
}

/// A log type used as a target for the filter.
#[derive(Debug, Default)]
pub struct Log;

impl Display for Log {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "log")
    }
}

impl BlockFiltering for Log {
    fn block_hash(&self) -> &'static str {
        "blockHash"
    }

    fn block_number(&self) -> &'static str {
        "blockNumber"
    }
}

impl LogFiltering for Log {
    fn address(&self) -> &'static str {
        "address"
    }
}

/// Builder for creating a filter on the Ethereum database.
#[derive(Debug, Default)]
pub struct EthDatabaseFilterBuilder<T: Default> {
    /// The filter to apply.
    pub filter: Document,
    /// The target type.
    pub target: T,
}

impl<T: BlockFiltering + Display + Default> EthDatabaseFilterBuilder<T> {
    /// Adds a filter on the block hash.
    #[must_use]
    pub fn with_block_hash(mut self, hash: &B256) -> Self {
        let key = format!("{}.{}", self.target, self.target.block_hash());
        self.filter.insert(key, format_hex(hash, HASH_HEX_STRING_LEN));
        self
    }

    /// Adds a filter on the block number.
    #[must_use]
    pub fn with_block_number(mut self, number: u64) -> Self {
        let key = format!("{}.{}", self.target, self.target.block_number());
        self.filter.insert(key, format_hex(number, BLOCK_NUMBER_HEX_STRING_LEN));
        self
    }

    /// Adds a filter on the block hash or number.
    #[must_use]
    pub fn with_block_hash_or_number(self, block_hash_or_number: BlockHashOrNumber) -> Self {
        match block_hash_or_number {
            BlockHashOrNumber::Hash(hash) => self.with_block_hash(&hash),
            BlockHashOrNumber::Number(number) => self.with_block_number(number),
        }
    }
}

impl<T: TransactionFiltering + Display + Default> EthDatabaseFilterBuilder<T> {
    /// Adds a filter on the transaction hash.
    #[must_use]
    pub fn with_tx_hash(mut self, hash: &B256) -> Self {
        let key = format!("{}.{}", self.target, self.target.transaction_hash());
        self.filter.insert(key, format_hex(hash, BLOCK_NUMBER_HEX_STRING_LEN));
        self
    }

    /// Adds a filter on the transaction index in the block.
    #[must_use]
    pub fn with_tx_index(mut self, index: &Index) -> Self {
        let index: usize = (*index).into();
        let key = format!("{}.{}", self.target, self.target.transaction_index());
        self.filter.insert(key, format_hex(index, U64_HEX_STRING_LEN));
        self
    }
}

impl<T: LogFiltering + BlockFiltering + Display + Default> EthDatabaseFilterBuilder<T> {
    /// Adds a filter on the log address.
    #[must_use]
    pub fn with_addresses(mut self, addresses: &[Address]) -> Self {
        if addresses.is_empty() {
            return self;
        }
        let key = format!("{}.{}", self.target, self.target.address());
        self.filter.insert(
            key,
            doc! {"$in": addresses.iter().map(|a| format_hex(a, ADDRESS_HEX_STRING_LEN)).collect::<Vec<_>>()},
        );
        self
    }

    /// Adds a filter on the block number range.
    #[must_use]
    pub fn with_block_number_range(mut self, from: u64, to: u64) -> Self {
        let key = format!("{}.{}", self.target, self.target.block_number());
        self.filter.insert(
            key,
            doc! {"$gte": format_hex(from, BLOCK_NUMBER_HEX_STRING_LEN), "$lte": format_hex(to, BLOCK_NUMBER_HEX_STRING_LEN)},
        );
        self
    }

    /// Adds a filter on the topics.
    #[must_use]
    pub fn with_topics(mut self, topics: &[Topic; 4]) -> Self {
        let mut filter = vec![];
        // If all topics are None, return a filter that checks if the log.topics field exists
        if topics.iter().all(Topic::is_empty) {
            self.filter.insert("log.topics", doc! {"$exists": true});
            return self;
        }

        // Iterate over the topics and add the filter to the filter vector
        for (index, topic_set) in topics.iter().enumerate() {
            let key = format!("log.topics.{index}");
            let topics: Vec<_> =
                topic_set.clone().into_iter().map(|t| format_hex(t, LOGS_TOPICS_HEX_STRING_LEN)).collect();

            if topics.len() == 1 {
                // If the topic array has only one element, use an equality filter
                filter.push(doc! {key: topics[0].clone()});
            } else if !topics.is_empty() {
                // If the topic array has more than one element, use an $in filter
                filter.push(doc! {key: {"$in": topics}});
            }
        }

        self.filter.extend(doc! {"$and": filter});
        self
    }
}

impl<T: Default> EthDatabaseFilterBuilder<T> {
    /// Consumes the builder and returns the filter and sorting.
    pub fn build(self) -> Document {
        self.filter
    }
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

/// Converts a key and value into a `MongoDB` filter.
pub fn into_filter<T>(key: &str, value: &T, width: usize) -> Document
where
    T: LowerHex,
{
    doc! {key: format_hex(value, width)}
}

#[cfg(test)]
mod tests {
    use super::*;
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
    fn test_transaction_block_hash_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Transaction>::default();

        // When
        let filter = builder.with_block_hash(&B256::left_padding_from(&[1])).build();

        // Then
        assert_eq!(filter, doc! {"tx.blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001"});
    }

    #[test]
    fn test_transaction_block_number_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Transaction>::default();

        // When
        let filter = builder.with_block_number(1).build();

        // Then
        assert_eq!(filter, doc! {"tx.blockNumber": "0x0000000000000001"});
    }

    #[test]
    fn test_transaction_block_hash_and_index_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Transaction>::default();

        // When
        let filter = builder.with_block_hash(&B256::left_padding_from(&[1])).with_tx_index(&10usize.into()).build();

        // Then
        assert_eq!(
            filter,
            doc! {
                "tx.blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001",
                "tx.transactionIndex": "0x000000000000000a"
            }
        );
    }

    #[test]
    fn test_transaction_block_number_and_index_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Transaction>::default();

        // When
        let filter = builder.with_block_number(1).with_tx_index(&10usize.into()).build();

        // Then
        assert_eq!(
            filter,
            doc! {
                "tx.blockNumber": "0x0000000000000001",
                "tx.transactionIndex": "0x000000000000000a"
            }
        );
    }

    #[test]
    fn test_receipt_transaction_hash_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Receipt>::default();

        // When
        let filter = builder.with_tx_hash(&B256::left_padding_from(&[1])).build();

        // Then
        assert_eq!(
            filter,
            doc! {"receipt.transactionHash": "0x0000000000000000000000000000000000000000000000000000000000000001"}
        );
    }

    #[test]
    fn test_receipt_block_number_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Receipt>::default();

        // When
        let filter = builder.with_block_number(1).build();

        // Then
        assert_eq!(filter, doc! {"receipt.blockNumber": "0x0000000000000001"});
    }

    #[test]
    fn test_receipt_block_hash_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Receipt>::default();

        // When
        let filter = builder.with_block_hash(&B256::left_padding_from(&[1])).build();

        // Then
        assert_eq!(
            filter,
            doc! {"receipt.blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001"}
        );
    }

    #[test]
    fn test_header_block_hash_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Header>::default();

        // When
        let filter = builder.with_block_hash(&B256::left_padding_from(&[1])).build();

        // Then
        assert_eq!(filter, doc! {"header.hash": "0x0000000000000000000000000000000000000000000000000000000000000001"});
    }

    #[test]
    fn test_header_block_number_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Header>::default();

        // When
        let filter = builder.with_block_number(1).build();

        // Then
        assert_eq!(filter, doc! {"header.number": "0x0000000000000001"});
    }

    #[test]
    fn test_log_block_hash_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();

        // When
        let filter = builder.with_block_hash(&B256::left_padding_from(&[1])).build();

        // Then
        assert_eq!(
            filter,
            doc! {"log.blockHash": "0x0000000000000000000000000000000000000000000000000000000000000001"}
        );
    }

    #[test]
    fn test_log_block_number_range_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();

        // When
        let filter = builder.with_block_number_range(1, 10).build();

        // Then
        assert_eq!(filter, doc! {"log.blockNumber": {"$gte": "0x0000000000000001", "$lte": "0x000000000000000a"}});
    }

    #[test]
    fn test_log_empty_addresses_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();

        // When
        let filter = builder.with_addresses(&[]).build();

        // Then
        assert_eq!(filter, doc! {});
    }

    #[test]
    fn test_log_addresses_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();

        // When
        let filter =
            builder.with_addresses(&[Address::left_padding_from(&[1]), Address::left_padding_from(&[2])]).build();

        // Then
        assert_eq!(
            filter,
            doc! {
                "log.address": {
                    "$in": ["0x0000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000002"]
                }
            }
        );
    }

    #[test]
    fn test_log_topics_empty_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();
        let topics = [Topic::default(), Topic::default(), Topic::default(), Topic::default()];

        // When
        let filter = builder.with_topics(&topics).build();

        // Then
        assert_eq!(filter, doc! { "log.topics": {"$exists": true} });
    }

    #[test]
    fn test_log_topics_filter() {
        // Given
        let builder = EthDatabaseFilterBuilder::<Log>::default();
        let topics: [FilterSet<B256>; 4] = [
            vec![B256::left_padding_from(&[1]), B256::left_padding_from(&[2])].into(),
            B256::left_padding_from(&[3]).into(),
            B256::left_padding_from(&[4]).into(),
            vec![B256::left_padding_from(&[5]), B256::left_padding_from(&[6])].into(),
        ];

        // When
        let filter = builder.with_topics(&topics).build();

        // Then
        let and_filter = filter.get("$and").unwrap().as_array().unwrap();
        let first_topic_filter = and_filter[0].as_document().unwrap().clone();
        assert!(
            first_topic_filter
                == doc! { "log.topics.0": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000001", "0x0000000000000000000000000000000000000000000000000000000000000002"]} }
                || first_topic_filter
                    == doc! { "log.topics.0": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000002", "0x0000000000000000000000000000000000000000000000000000000000000001"]} }
        );
        assert_eq!(
            and_filter[1].as_document().unwrap().clone(),
            doc! { "log.topics.1": "0x0000000000000000000000000000000000000000000000000000000000000003" }
        );
        assert_eq!(
            and_filter[2].as_document().unwrap().clone(),
            doc! { "log.topics.2": "0x0000000000000000000000000000000000000000000000000000000000000004" }
        );
        let fourth_topic_filter = and_filter[3].as_document().unwrap().clone();
        assert!(
            fourth_topic_filter
                == doc! { "log.topics.3": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000005", "0x0000000000000000000000000000000000000000000000000000000000000006"]} }
                || fourth_topic_filter
                    == doc! { "log.topics.3": {"$in": ["0x0000000000000000000000000000000000000000000000000000000000000006", "0x0000000000000000000000000000000000000000000000000000000000000005"]} }
        );
    }
}
