use reth_primitives::U256;
use reth_rpc_types::{Filter, ValueOrArray};
use starknet::core::types::{BlockId, EventFilter};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use super::block::EthBlockNumberOrTag;
use super::felt::Felt252Wrapper;
use crate::client::api::KakarotStarknetApi;
use crate::client::errors::EthApiError;
use crate::client::helpers::split_u256_into_field_elements;
use crate::client::KakarotClient;

pub struct EthEventFilter(Filter);

impl From<Filter> for EthEventFilter {
    fn from(filter: Filter) -> Self {
        Self(filter)
    }
}

impl From<EthEventFilter> for Filter {
    fn from(filter: EthEventFilter) -> Self {
        filter.0
    }
}

impl EthEventFilter {
    pub fn to_starknet_filter<P: Provider + Send + Sync>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<EventFilter, EthApiError<P::Error>> {
        let filter: Filter = self.into();
        let block_hash = filter.get_block_hash();

        // Extract keys into topics
        let mut keys: Vec<FieldElement> = filter
            .topics()
            .flat_map(|topic| match topic {
                ValueOrArray::Value(value) => match value {
                    None => vec![],
                    Some(topic) => {
                        let topic = U256::from_be_bytes(topic.to_fixed_bytes());
                        split_u256_into_field_elements(topic).to_vec()
                    }
                },
                ValueOrArray::Array(topics) => topics
                    .iter()
                    .filter_map(|topic| {
                        topic.map(|topic| {
                            let topic = U256::from_be_bytes(topic.to_fixed_bytes());
                            split_u256_into_field_elements(topic).to_vec()
                        })
                    })
                    .flatten()
                    .collect(),
            })
            .take(8) // take up to 4 topics split into 2 field elements
            .collect();

        // Get the filter address if any
        if let Some(address) = filter.address {
            let address = match address {
                ValueOrArray::Array(addresses) => addresses.first().copied(),
                ValueOrArray::Value(address) => Some(address),
            };
            if let Some(address) = address {
                let address: Felt252Wrapper = address.into();
                keys.append(&mut vec![address.into()])
            }
        }

        let keys = if !keys.is_empty() { Some(keys.into_iter().map(|key| vec![key]).collect()) } else { None };

        let starknet_filter = if let Some(block_hash) = block_hash {
            let block_hash: Felt252Wrapper = block_hash.try_into()?;

            EventFilter {
                from_block: Some(BlockId::Hash(block_hash.clone().into())),
                to_block: Some(BlockId::Hash(block_hash.into())),
                address: Some(client.kakarot_address()),
                keys,
            }
        } else {
            let from_block = filter.block_option.get_from_block().copied().map(Into::<EthBlockNumberOrTag>::into);
            let to_block = filter.block_option.get_to_block().copied().map(Into::<EthBlockNumberOrTag>::into);
            EventFilter {
                from_block: from_block.map(Into::<BlockId>::into),
                to_block: to_block.map(Into::<BlockId>::into),
                address: Some(client.kakarot_address()),
                keys,
            }
        };

        Ok(starknet_filter)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::mock::constants::KAKAROT_ADDRESS;
    use crate::mock::mock_starknet::{fixtures, init_mock_client};

    fn assert_eq_event_filter(lhs: EventFilter, rhs: EventFilter) {
        assert_eq!(lhs.from_block, rhs.from_block);
        assert_eq!(lhs.to_block, rhs.to_block);
        assert_eq!(lhs.address, Some(*KAKAROT_ADDRESS));
        assert_eq!(lhs.keys, rhs.keys);
    }

    #[tokio::test]
    async fn test_to_starknet_event_filter_with_block_hash() {
        // Given
        let eth_event_filter: Filter =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_filter_block_hash.json")).unwrap();
        let eth_event_filter: EthEventFilter = eth_event_filter.into();

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let starknet_event_filter = eth_event_filter.to_starknet_filter(&client).unwrap();

        // Then
        let expected: EventFilter =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_filter_block_hash.json")).unwrap();
        assert_eq_event_filter(expected, starknet_event_filter);
    }

    #[tokio::test]
    async fn test_to_starknet_event_filter_with_from_to() {
        // Given
        let eth_event_filter: Filter =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_filter_from_to.json")).unwrap();
        let eth_event_filter: EthEventFilter = eth_event_filter.into();

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let starknet_event_filter = eth_event_filter.to_starknet_filter(&client).unwrap();

        // Then
        let expected: EventFilter =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_filter_from_to.json")).unwrap();
        assert_eq_event_filter(expected, starknet_event_filter);
    }

    #[tokio::test]
    async fn test_to_starknet_event_filter_without_topics() {
        // Given
        let eth_event_filter: Filter =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_filter_without_topics.json")).unwrap();
        let eth_event_filter: EthEventFilter = eth_event_filter.into();

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let starknet_event_filter = eth_event_filter.to_starknet_filter(&client).unwrap();

        // Then
        let expected: EventFilter =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_filter_without_topics.json"))
                .unwrap();
        assert_eq_event_filter(expected, starknet_event_filter);
    }

    #[tokio::test]
    async fn test_to_starknet_event_filter_without_address() {
        // Given
        let eth_event_filter: Filter =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_filter_without_address.json")).unwrap();
        let eth_event_filter: EthEventFilter = eth_event_filter.into();

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let starknet_event_filter = eth_event_filter.to_starknet_filter(&client).unwrap();

        // Then
        let expected: EventFilter =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_filter_without_address.json"))
                .unwrap();
        assert_eq_event_filter(expected, starknet_event_filter);
    }

    #[tokio::test]
    async fn test_to_starknet_event_filter_without_topics_or_address() {
        // Given
        let eth_event_filter: Filter =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_filter_without_topics_or_address.json"))
                .unwrap();
        let eth_event_filter: EthEventFilter = eth_event_filter.into();

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let starknet_event_filter = eth_event_filter.to_starknet_filter(&client).unwrap();

        // Then
        let expected: EventFilter = serde_json::from_str(include_str!(
            "test_data/conversion/starknet/event_filter_without_topics_or_address.json"
        ))
        .unwrap();
        assert_eq_event_filter(expected, starknet_event_filter);
    }
}
