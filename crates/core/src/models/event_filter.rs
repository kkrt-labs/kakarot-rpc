use reth_primitives::U256;
use reth_rpc_types::{Filter, ValueOrArray};
use starknet::core::types::{BlockId, EventFilter};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

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
        let mut starknet_filter = if let Some(block_hash) = filter.get_block_hash() {
            let block_hash: Felt252Wrapper = block_hash.try_into()?;

            EventFilter {
                from_block: Some(BlockId::Hash(block_hash.clone().into())),
                to_block: Some(BlockId::Hash(block_hash.into())),
                address: Some(client.kakarot_address()),
                keys: None,
            }
        } else {
            let from_block = filter.get_from_block().map(BlockId::Number);
            let to_block = filter.get_to_block().map(BlockId::Number);
            EventFilter { from_block, to_block, address: Some(client.kakarot_address()), keys: None }
        };

        let keys: Vec<FieldElement> = filter
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

        starknet_filter.keys =
            if !keys.is_empty() { Some(keys.into_iter().map(|key| vec![key]).collect()) } else { None };

        Ok(starknet_filter)
    }
}
