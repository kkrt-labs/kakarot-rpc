use core::iter::once;

use num_bigint::BigUint;
use reth_primitives::{Address, Bytes, H256, U256};
use reth_rpc_types::Log;
use starknet::core::types::Event;
use starknet::providers::Provider;

use super::felt::Felt252Wrapper;
use crate::client::api::KakarotStarknetApi;
use crate::client::errors::EthApiError;
use crate::client::helpers::vec_felt_to_bytes;
use crate::models::convertible::ConvertibleStarknetEvent;

#[derive(Debug, Clone)]
pub struct StarknetEvent(Event);

impl StarknetEvent {
    pub fn new(sn_event: Event) -> Self {
        Self(sn_event)
    }
}

impl From<Event> for StarknetEvent {
    fn from(event: Event) -> Self {
        Self::new(event)
    }
}

impl ConvertibleStarknetEvent for StarknetEvent {
    fn to_eth_log<P: Provider + Send + Sync>(
        self,
        client: &dyn KakarotStarknetApi<P>,
        block_hash: Option<H256>,
        block_number: Option<U256>,
        transaction_hash: Option<H256>,
        log_index: Option<U256>,
        transaction_index: Option<U256>,
    ) -> Result<Log, EthApiError<P::Error>> {
        // If event `from_address` does not equal kakarot address, return early
        if self.0.from_address != client.kakarot_address() {
            return Err(EthApiError::KakarotDataFilteringError("Event".into()));
        }

        // Derive the evm address from the last item in the `event.keys` vector and remove it
        let (evm_contract_address, keys) =
            self.0.keys.split_last().ok_or_else(|| EthApiError::KakarotDataFilteringError("Event".into()))?;

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

        let data: Bytes = vec_felt_to_bytes(self.0.data);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::mock_starknet::{fixtures, init_mock_client};

    #[test]
    fn test_to_eth_log_log3() {
        // Given
        let event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log3.json")).unwrap();
        let starknet_event = StarknetEvent::new(event);

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let eth_log = starknet_event.to_eth_log(&client, None, None, None, None, None).unwrap();

        // Then
        let expected: Log = serde_json::from_str(include_str!("test_data/conversion/eth/event_log3.json")).unwrap();
        assert_eq!(expected, eth_log);
    }

    #[test]
    fn test_to_eth_log_log4() {
        // Given
        let event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log4.json")).unwrap();
        let starknet_event = StarknetEvent::new(event);

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let eth_log = starknet_event.to_eth_log(&client, None, None, None, None, None).unwrap();

        // Then
        let expected: Log = serde_json::from_str(include_str!("test_data/conversion/eth/event_log4.json")).unwrap();
        assert_eq!(expected, eth_log);
    }

    #[test]
    #[should_panic(expected = "KakarotDataFilteringError(\"Event\")")]
    fn test_to_eth_log_should_fail_on_from_address_not_kakarot_address() {
        // Given
        let event: Event =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_invalid_from_address.json"))
                .unwrap();
        let starknet_event = StarknetEvent::new(event);

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        starknet_event.to_eth_log(&client, None, None, None, None, None).unwrap();
    }

    #[test]
    #[should_panic(expected = "ConversionError(\"failed to convert Felt252Wrapper to Ethereum address: the value \
                               exceeds the maximum size of an Ethereum address\")")]
    fn test_to_eth_log_should_fail_on_key_not_convertible_to_eth_address() {
        // Given
        let event: Event =
            serde_json::from_str(include_str!("test_data/conversion/starknet/event_invalid_key.json")).unwrap();
        let starknet_event = StarknetEvent::new(event);

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        starknet_event.to_eth_log(&client, None, None, None, None, None).unwrap();
    }

    #[test]
    fn test_to_eth_log_with_optional_parameters() {
        // Given
        let event: Event = serde_json::from_str(include_str!("test_data/conversion/starknet/event_log3.json")).unwrap();
        let starknet_event = StarknetEvent::new(event);

        let fixtures = fixtures(vec![]);
        let client = init_mock_client(Some(fixtures));

        // When
        let block_hash = Some(H256::from_low_u64_be(0xdeadbeef));
        let block_number = Some(U256::from(0x1));
        let transaction_hash = Some(H256::from_low_u64_be(0x12));
        let transaction_index = Some(U256::from(0x123));
        let log_index = Some(U256::from(0x1234));
        let eth_event = starknet_event
            .to_eth_log(&client, block_hash, block_number, transaction_hash, log_index, transaction_index)
            .unwrap();

        // Then
        let expected: Log =
            serde_json::from_str(include_str!("test_data/conversion/eth/event_log3_with_optionals.json")).unwrap();
        assert_eq!(expected, eth_event);
    }
}
