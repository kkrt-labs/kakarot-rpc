use async_trait::async_trait;
use reth_primitives::{Bloom, H256, U128, U256, U64, U8};
use reth_rpc_types::TransactionReceipt as EthTransactionReceipt;
use starknet::core::types::{
    ExecutionResult, InvokeTransactionReceipt, MaybePendingTransactionReceipt, TransactionReceipt,
};
use starknet::providers::Provider;

use super::convertible::{
    ConvertibleStarknetEvent, ConvertibleStarknetTransaction, ConvertibleStarknetTransactionReceipt,
};
use super::event::StarknetEvent;
use super::felt::Felt252Wrapper;
use super::transaction::block_tx::StarknetBlockTransaction;
use crate::client::api::KakarotStarknetApi;
use crate::client::constants::selectors::EVM_CONTRACT_DEPLOYED;
use crate::client::errors::EthApiError;
use crate::client::helpers::DataDecodingError;
use crate::client::KakarotClient;

pub struct StarknetTransactionReceipt(MaybePendingTransactionReceipt);

impl From<MaybePendingTransactionReceipt> for StarknetTransactionReceipt {
    fn from(receipt: MaybePendingTransactionReceipt) -> Self {
        Self(receipt)
    }
}

impl From<StarknetTransactionReceipt> for MaybePendingTransactionReceipt {
    fn from(receipt: StarknetTransactionReceipt) -> Self {
        receipt.0
    }
}

#[async_trait]
impl ConvertibleStarknetTransactionReceipt for StarknetTransactionReceipt {
    async fn to_eth_transaction_receipt<P: Provider + Send + Sync + 'static>(
        self,
        client: &KakarotClient<P>,
    ) -> Result<Option<EthTransactionReceipt>, EthApiError<P::Error>> {
        let starknet_tx_receipt: MaybePendingTransactionReceipt = self.into();

        let res_receipt = match starknet_tx_receipt {
            MaybePendingTransactionReceipt::Receipt(receipt) => match receipt {
                TransactionReceipt::Invoke(InvokeTransactionReceipt {
                    transaction_hash,
                    execution_result,
                    block_hash,
                    block_number,
                    events,
                    ..
                }) => {
                    let starknet_tx: StarknetBlockTransaction =
                        client.starknet_provider().get_transaction_by_hash(transaction_hash).await?.into();

                    let transaction_hash: Felt252Wrapper = transaction_hash.into();
                    let transaction_hash: Option<H256> = Some(transaction_hash.into());

                    let block_hash: Felt252Wrapper = block_hash.into();
                    let block_hash: Option<H256> = Some(block_hash.into());

                    let block_number: Felt252Wrapper = block_number.into();
                    let block_number: Option<U256> = Some(block_number.into());

                    let eth_tx = starknet_tx.to_eth_transaction(client, block_hash, block_number, None).await?;
                    let from = eth_tx.from;
                    let to = eth_tx.to;
                    let contract_address = match to {
                        // If to is Some, means contract_address should be None as it is a normal transaction
                        Some(_) => None,
                        // If to is None, is a contract creation transaction so contract_address should be Some
                        None => {
                            let event = events
                                .iter()
                                .find(|event| event.keys.iter().any(|key| *key == EVM_CONTRACT_DEPLOYED))
                                .ok_or(EthApiError::Other(anyhow::anyhow!(
                                    "Kakarot Core: No contract deployment event found in Kakarot transaction receipt"
                                )))?;

                            let evm_address =
                                event.data.first().ok_or(DataDecodingError::InvalidReturnArrayLength {
                                    entrypoint: "deployment".into(),
                                    expected: 1,
                                    actual: 0,
                                })?;

                            let evm_address = Felt252Wrapper::from(*evm_address);
                            Some(evm_address.try_into()?)
                        }
                    };

                    let status_code = match execution_result {
                        ExecutionResult::Succeeded => Some(U64::from(1)),
                        ExecutionResult::Reverted { .. } => Some(U64::from(0)),
                    };

                    let logs = events
                        .into_iter()
                        .map(StarknetEvent::new)
                        .filter_map(|event| {
                            event.to_eth_log(client, block_hash, block_number, transaction_hash, None, None).ok()
                        })
                        .collect();

                    EthTransactionReceipt {
                        transaction_hash,
                        // TODO: transition this hardcoded default out of nearing-demo-day hack and seeing how to
                        // properly source/translate this value
                        transaction_index: U64::from(0), // TODO: Fetch real data
                        block_hash,
                        block_number,
                        from,
                        to,
                        cumulative_gas_used: U256::from(1_000_000), // TODO: Fetch real data
                        gas_used: Some(U256::from(500_000)),
                        contract_address,
                        logs,
                        state_root: None,             // TODO: Fetch real data
                        logs_bloom: Bloom::default(), // TODO: Fetch real data
                        status_code,
                        effective_gas_price: U128::from(1_000_000), // TODO: Fetch real data
                        transaction_type: U8::from(0),              // TODO: Fetch real data
                    }
                }
                // L1Handler, Declare, Deploy and DeployAccount transactions unsupported for now in
                // Kakarot
                _ => return Ok(None),
            },
            MaybePendingTransactionReceipt::PendingReceipt(_) => {
                return Ok(None);
            }
        };

        Ok(Some(res_receipt))
    }
}

#[cfg(test)]
mod tests {
    use starknet::providers::jsonrpc::JsonRpcMethod;

    use super::*;
    use crate::mock::constants::{ABDEL_STARKNET_ADDRESS_HEX, PROXY_ACCOUNT_CLASS_HASH_HEX};
    use crate::mock::mock_starknet::{fixtures, init_mock_client, AvailableFixtures};
    use crate::wrap_kakarot;

    #[tokio::test]
    async fn test_to_eth_transaction_receipt() {
        // Given
        let starknet_transaction_receipt: MaybePendingTransactionReceipt =
            serde_json::from_str(include_str!("test_data/conversion/starknet/transaction_receipt.json")).unwrap();
        let starknet_transaction_receipt: StarknetTransactionReceipt = starknet_transaction_receipt.into();

        let fixtures = fixtures(vec![
            AvailableFixtures::GetClassHashAt(ABDEL_STARKNET_ADDRESS_HEX.into(), PROXY_ACCOUNT_CLASS_HASH_HEX.into()),
            AvailableFixtures::GetEvmAddress,
            wrap_kakarot!(JsonRpcMethod::GetTransactionByHash),
        ]);
        let client = init_mock_client(Some(fixtures));

        // When
        let eth_transaction = starknet_transaction_receipt.to_eth_transaction_receipt(&client).await.unwrap();

        // Then
        let expected: EthTransactionReceipt =
            serde_json::from_str(include_str!("test_data/conversion/eth/transaction_receipt.json")).unwrap();
        assert_eq!(Some(expected), eth_transaction);
    }
}
