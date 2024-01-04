use core::iter::once;

use num_bigint::BigUint;
use reth_primitives::{Address, Bytes, H256, U256};
use reth_rpc_types::Log;
use starknet::core::types::Event;
use starknet::providers::Provider;
use starknet_crypto::FieldElement;

use super::felt::Felt252Wrapper;
use crate::starknet_client::errors::EthApiError;
use crate::starknet_client::KakarotClient;

#[derive(Debug, Clone)]
pub struct StarknetEvent(Event);

impl StarknetEvent {
    pub const fn new(sn_event: Event) -> Self {
        Self(sn_event)
    }
}

impl From<Event> for StarknetEvent {
    fn from(event: Event) -> Self {
        Self::new(event)
    }
}

impl StarknetEvent {
    pub fn to_eth_log<P: Provider + Send + Sync>(
        self,
        client: &KakarotClient<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError> {
        // If event `from_address` does not equal kakarot address, return early
        if self.0.from_address != client.kakarot_address() {
            return Err(EthApiError::KakarotDataFilteringError("Event".into()));
        }

        // Derive the evm address from the first item in the `event.keys` vector and remove it
        let (evm_contract_address, keys) =
            self.0.keys.split_first().ok_or_else(|| EthApiError::KakarotDataFilteringError("Event".into()))?;

        let address: Address = {
            let felt_wrapper: Felt252Wrapper = (*evm_contract_address).into();
            felt_wrapper.try_into()?
        };

        let topics: Vec<H256> = keys
            .chunks(2)
            .map(|chunk| {
                let low = BigUint::from_bytes_be(&chunk[0].to_bytes_be());
                let high = match chunk.get(1) {
                    Some(h) => BigUint::from_bytes_be(&h.to_bytes_be()),
                    None => {
                        return Err(anyhow::anyhow!("Not a convertible event: High value doesn't exist",));
                    }
                };
                let result = low + (BigUint::from(2u128).pow(128u32) * high);
                // Converts the result to bytes.
                let bytes = result.to_bytes_be();
                // If the length of bytes is less than 32, prepends it with zeros to make it 32 bytes long.
                let bytes = once(0u8).cycle().take(32 - bytes.len()).chain(bytes.into_iter()).collect::<Vec<_>>();
                Ok(H256::from_slice(&bytes))
            })
            .collect::<Result<_, _>>()?;

        let data =
            Bytes::from(self.0.data.into_iter().filter_map(|x: FieldElement| u8::try_from(x).ok()).collect::<Vec<_>>());

        Ok(Log {
            address,
            topics,
            data,
            block_hash,
            block_number,
            transaction_hash,
            log_index,
            transaction_index,
            removed: false,
        })
    }
}
