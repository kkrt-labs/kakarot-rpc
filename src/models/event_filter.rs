use reth_primitives::U256;
use reth_rpc_types::{Filter, ValueOrArray};
use starknet::core::types::{BlockId, EventFilter};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use super::block::EthBlockNumberOrTag;
use super::felt::Felt252Wrapper;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::helpers::split_u256_into_field_elements;
use crate::starknet_client::KakarotClient;

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
    pub fn to_starknet_event_filter<P: Provider + Send + Sync>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<EventFilter, EthApiError> {
        let filter: Filter = self.into();
        let block_hash = filter.get_block_hash();

        // Extract keys into topics
        let mut keys: Vec<FieldElement> = filter
            .topics
            .into_iter()
            .map(|topic| topic.to_value_or_array())
            .flat_map(|topic| match topic {
                None => vec![],
                Some(ValueOrArray::Value(value)) => {
                    let topic = U256::from_be_bytes(value.to_fixed_bytes());
                    split_u256_into_field_elements(topic).to_vec()
                },
                Some(ValueOrArray::Array(topics)) => topics
                .iter()
                .flat_map(|topic| {
                    let topic = U256::from_be_bytes(topic.to_fixed_bytes());
                    split_u256_into_field_elements(topic).to_vec()
                })
                .collect(),
            })
            .take(8) // take up to 4 topics split into 2 field elements
            .collect();

        // Get the filter address if any (added as first key)
        if let Some(address) = filter.address.to_value_or_array() {
            let address = match address {
                ValueOrArray::Array(addresses) => addresses.first().copied(),
                ValueOrArray::Value(address) => Some(address),
            };
            if let Some(address) = address {
                let address: Felt252Wrapper = address.into();
                keys = [vec![address.into()], keys].concat();
            }
        }

        // Convert to expected format Vec<Vec<FieldElement>> or None if no keys
        let keys = if !keys.is_empty() { Some(keys.into_iter().map(|key| vec![key]).collect()) } else { None };

        // Add filter block range
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
