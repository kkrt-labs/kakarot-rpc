mod test {
    use kakarot_rpc_core::lightclient::{
        types::{Block, BlockTransactions, Header, Rich},
        MockStarknetClient, StarknetClient,
    };

    use reth_primitives::{Address, Bloom, Bytes, H160, H256, H64, U256};
    use reth_rpc_types::SyncStatus;
    use starknet::providers::jsonrpc::models::BlockId as StarknetBlockId;

    use std::collections::BTreeMap;

    #[tokio::test]
    async fn when_call_block_number_return_ok() {
        // Given
        // Mock config, ethereum light client and starknet light client.
        let mut starknet_lightclient_mock = config();

        // Set expect to testing RPC method
        starknet_lightclient_mock
            .expect_block_number()
            .returning(|| Ok(1));

        let result_mock = starknet_lightclient_mock.block_number().await;

        // assert!(result_lightclient.is_ok());
        // Then
        assert_eq!(1, result_mock.unwrap());
    }

    #[tokio::test]
    async fn when_call_syncing_return_ok() {
        // Given
        // Mock config, ethereum light client and starknet light client.
        let mut starknet_lightclient_mock = config();

        // Set expect to testing RPC method
        starknet_lightclient_mock
            .expect_syncing()
            .returning(|| Ok(SyncStatus::None));

        let result_mock = starknet_lightclient_mock.syncing().await;

        assert_eq!(SyncStatus::None, result_mock.unwrap());
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

        // When
        let result = starknet_lightclient_mock
            .get_code(ethereum_address, starknet_block_id)
            .await;
        assert!(result.is_ok());

        let result_value = result.unwrap();

        // Then
        assert_eq!(bytes_result_value, result_value);
    }

    #[tokio::test]
    async fn when_get_block_with_tx_hashes_then_should_return_bytes() {
        // Given
        let mut starknet_lightclient_mock = config();

        let hash = H256::default();
        let parent_hash = H256::default();
        let sequencer = H160::default();
        let state_root = H256::default();
        let number = U256::from(100);
        let timestamp = U256::from(100);
        let transactions = BlockTransactions::Hashes(vec![]);
        let gas_limit = U256::ZERO;
        let gas_used = U256::ZERO;
        let difficulty = U256::ZERO;
        let nonce: Option<H64> = None;
        let size: Option<U256> = None;
        // Bloom is a byte array of length 256
        let logs_bloom = Bloom::default();
        let extra_data = Bytes::from(b"0x00");
        let total_difficulty: U256 = U256::ZERO;
        let mix_hash = H256::default();
        let base_fee_per_gas = U256::ZERO;
        let header = Header {
            hash: Some(hash),
            parent_hash,
            uncles_hash: parent_hash,
            author: sequencer,
            miner: sequencer,
            state_root,
            // BlockWithTxHashes doesn't have a transactions root
            transactions_root: H256::zero(),
            // BlockWithTxHashes doesn't have a receipts root
            receipts_root: H256::zero(),
            number: Some(number),
            gas_used,
            gas_limit,
            extra_data,
            logs_bloom,
            timestamp,
            difficulty,
            nonce,
            size,
            mix_hash,
            base_fee_per_gas,
        };
        let block = Block {
            header,
            total_difficulty,
            uncles: vec![],
            transactions,
            base_fee_per_gas: None,
            size,
        };
        let expected_result = Rich::<Block> {
            inner: block,
            extra_info: BTreeMap::default(),
        };

        let expected_result_value = expected_result.clone();

        starknet_lightclient_mock
            .expect_get_eth_block_from_starknet_block()
            .returning(move |_, _| Ok(expected_result.clone()));

        let block_id = StarknetBlockId::Number(0);
        let hydrated_tx = false;
        // When
        let result = starknet_lightclient_mock
            .get_eth_block_from_starknet_block(block_id, hydrated_tx)
            .await;
        assert!(result.is_ok());

        let result_value = result.unwrap();

        // Then
        assert_eq!(expected_result_value, result_value);
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
