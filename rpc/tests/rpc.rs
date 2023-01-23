#[cfg(test)]
mod test {
    use kakarot_rpc::eth_rpc::KakarotEthRpc;
    use kakarot_rpc_core::lightclient::{MockStarknetClient, StarknetClient, StarknetClientImpl};
    use reth_primitives::{rpc::BlockId, Address, Bytes, H256, U256};
    use reth_rpc_types::{Block, BlockTransactions, Header, RichBlock};
    use starknet::{
        core::types::FieldElement,
        macros::selector,
        providers::jsonrpc::{
            models::{BlockId as StarknetBlockId, FunctionCall},
            HttpTransport, JsonRpcClient, JsonRpcClientError,
        },
    };

    #[tokio::test]
    async fn when_call_block_number_return_ok() {
        // Given
        // Mock config, ethereum light client and starknet light client.
        let mut starknet_lightclient_mock = config();

        // Set expect to testing RPC method
        starknet_lightclient_mock
            .expect_block_number()
            .returning(|| Ok(1));

        let kakarot_rpc_mock = KakarotEthRpc::new(Box::new(starknet_lightclient_mock));

        let result_mock = kakarot_rpc_mock.starknet_client.block_number().await;

        // Then
        assert_eq!(1, result_mock.unwrap());
    }

    #[tokio::test]
    async fn when_get_code_then_should_return_bytes() {
        // Given
        let mut starknet_lightclient_mock = config();
        let starknet_block_id = StarknetBlockId::Number(1);
        let ethereum_address = Address::from_slice(&[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
        ]);
        let bytes = vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
        ];
        let bytes_result = Bytes::from(bytes.clone());
        let bytes_result_value = bytes_result.clone();

        starknet_lightclient_mock
            .expect_get_code()
            .returning(move |_, _| Ok(bytes_result.clone()));

        let kakarot_rpc_mock = KakarotEthRpc::new(Box::new(starknet_lightclient_mock));

        // When
        let result = kakarot_rpc_mock
            .starknet_client
            .get_code(ethereum_address, starknet_block_id)
            .await;
        assert!(result.is_ok());

        let result_value = result.unwrap();
        // Then
        assert_eq!(
            format!("{bytes_result_value:?}"),
            format!("{result_value:?}")
        );
    }

    fn config() -> MockStarknetClient {
        // Given
        // Mock config, ethereum light client and starknet light client.
        let mut starknet_lightclient_mock = MockStarknetClient::new();

        // Set expect to testing RPC method
        starknet_lightclient_mock
            .expect_block_number()
            .returning(|| Ok(1));

        // Set lightclient

        starknet_lightclient_mock
    }
}
