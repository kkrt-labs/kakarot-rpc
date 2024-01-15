use reth_primitives::U256;
use reth_rpc_types::{Filter, ValueOrArray};
use starknet::core::types::{BlockId, EventFilter};
use starknet::providers::Provider;
use starknet_crypto::FieldElement;
use tracing::debug;

use super::block::EthBlockNumberOrTag;
use super::felt::Felt252Wrapper;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::helpers::split_u256;
use crate::starknet_client::KakarotClient;
use crate::{into_via_try_wrapper, into_via_wrapper};

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
    #[tracing::instrument(skip_all, level = "debug")]
    pub fn to_starknet_event_filter<P: Provider + Send + Sync>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<EventFilter, EthApiError> {
        let filter: Filter = self.into();
        let block_hash = filter.get_block_hash();

        debug!("ethereum event filter: {:?}", filter);

        // Extract keys into topics
        let keys: Vec<FieldElement> = filter
            .topics
            .into_iter()
            .flat_map(|filter| match filter.to_value_or_array() {
                None => vec![],
                Some(ValueOrArray::Value(value)) => {
                    let topic = U256::from_be_bytes(value.to_fixed_bytes());
                    split_u256(topic).to_vec()
                },
                Some(ValueOrArray::Array(topics)) => topics
                .iter()
                .flat_map(|topic| {
                    let topic = U256::from_be_bytes(topic.to_fixed_bytes());
                    split_u256(topic).to_vec()
                })
                .collect(),
            })
            .take(8) // take up to 4 topics split into 2 field elements
            .collect();

        // Get the filter address if any (added as first key)
        let address = filter.address.to_value_or_array().and_then(|a| match a {
            ValueOrArray::Array(addresses) => addresses.first().copied(),
            ValueOrArray::Value(address) => Some(address),
        });

        // Convert to expected format Vec<Vec<FieldElement>> or None if no keys and no address
        let keys = if !keys.is_empty() | address.is_some() {
            let keys = keys.into_iter().map(|key| vec![key]).collect();
            // If address is present add it as first key, otherwise add an empty key
            let keys = [address.map_or(vec![vec![]], |a| vec![vec![into_via_wrapper!(a)]]), keys].concat();
            Some(keys)
        } else {
            None
        };

        debug!("starknet event filter keys: {:?}", keys);

        // Add filter block range
        let starknet_filter = if let Some(block_hash) = block_hash {
            let block_hash = into_via_try_wrapper!(block_hash);

            EventFilter {
                from_block: Some(BlockId::Hash(block_hash)),
                to_block: Some(BlockId::Hash(block_hash)),
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

        debug!("starknet event filter: {:?}", starknet_filter);

        Ok(starknet_filter)
    }
}
