mod tests {

    use std::str::FromStr;

    use kakarot_rpc_core::client::api::KakarotEthApi;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS_EVM;
    use kakarot_rpc_core::models::balance::{TokenBalance, TokenBalances};
    use kakarot_test_utils::deploy_helpers::KakarotTestEnvironmentContext;
    use kakarot_test_utils::execution_helpers::execute_eth_tx;
    use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
    use reth_primitives::{BlockId, BlockNumberOrTag, Bytes, H256, U256};
    use reth_rpc_types::{Filter, FilterBlockOption, FilterChanges, Log, ValueOrArray};
    use rstest::*;
    use starknet::core::types::FieldElement;

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_eoa_balance(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot) = kakarot_test_env_ctx.resources();

        // When
        let eoa_balance = client
            .balance(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let eoa_balance = FieldElement::from_bytes_be(&eoa_balance.to_be_bytes()).unwrap();

        // Then
        assert_eq!(FieldElement::from_dec_str("1000000000000000000").unwrap(), eoa_balance);
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_token_balances(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot, _, erc20_eth_address) = kakarot_test_env_ctx.resources_with_contract("ERC20");

        // When
        let to = U256::try_from_be_slice(&kakarot.eoa_addresses.eth_address.to_fixed_bytes()[..]).unwrap();
        let amount = U256::from(10_000);
        execute_eth_tx(&kakarot_test_env_ctx, "ERC20", "mint", vec![to, amount]).await;

        // Then
        let balances = client.token_balances(kakarot.eoa_addresses.eth_address, vec![erc20_eth_address]).await.unwrap();
        assert_eq!(
            TokenBalances {
                address: kakarot.eoa_addresses.eth_address,
                token_balances: vec![TokenBalance {
                    token_address: erc20_eth_address,
                    token_balance: Some(U256::from(10_000)),
                    error: None
                }]
            },
            balances
        );
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_storage_at(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, _, _, counter_eth_address) = kakarot_test_env_ctx.resources_with_contract("Counter");
        // When
        execute_eth_tx(&kakarot_test_env_ctx, "Counter", "inc", vec![]).await;

        // Then
        let count = client
            .storage_at(counter_eth_address, U256::from(0), BlockId::Number(BlockNumberOrTag::Latest))
            .await
            .unwrap();
        assert_eq!(U256::from(1), count);
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_logs(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot, _, erc20_eth_address) = kakarot_test_env_ctx.resources_with_contract("ERC20");

        // When
        let to = U256::try_from_be_slice(&kakarot.eoa_addresses.eth_address.to_fixed_bytes()[..]).unwrap();
        let amount = U256::from(10_000);
        let mint_tx_hash = execute_eth_tx(&kakarot_test_env_ctx, "ERC20", "mint", vec![to, amount]).await;

        let to = U256::try_from_be_slice(ACCOUNT_ADDRESS_EVM.as_bytes()).unwrap();
        let amount = U256::from(10_000);
        let transfer_tx_hash = execute_eth_tx(&kakarot_test_env_ctx, "ERC20", "transfer", vec![to, amount]).await;

        let filter = Filter {
            block_option: FilterBlockOption::Range {
                from_block: Some(BlockNumberOrTag::Number(0)),
                to_block: Some(BlockNumberOrTag::Number(100)),
            },
            address: ValueOrArray::Value(erc20_eth_address).into(),
            topics: [
                ValueOrArray::Value(None).into(),
                ValueOrArray::Value(None).into(),
                ValueOrArray::Value(None).into(),
                ValueOrArray::Value(None).into(),
            ],
        };
        let logs = client.get_logs(filter).await.unwrap();

        // Then
        match logs {
            FilterChanges::Logs(logs) => {
                assert_eq!(2, logs.len());
                assert_eq!(
                    Log {
                        address: erc20_eth_address,
                        topics: vec![
                            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                                .unwrap(), // keccak256(“Transfer(address,address,uint256)”)
                            H256::from_low_u64_be(0u64),                   // from
                            H256::from(kakarot.eoa_addresses.eth_address)  // to
                        ],
                        data: Bytes::from_str("0x0000000000000000000000000000000000000000000000000000000000002710")
                            .unwrap(), // amount
                        block_hash: logs[0].block_hash, // block hash changes so just set to event value
                        block_number: logs[0].block_number, // block number changes so just set to event value
                        transaction_hash: Some(mint_tx_hash),
                        transaction_index: None,
                        log_index: None,
                        removed: false
                    },
                    logs[0]
                );
                assert_eq!(
                    Log {
                        address: erc20_eth_address,
                        topics: vec![
                            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                                .unwrap(), // keccak256("Transfer(address,address,uint256)")
                            H256::from(kakarot.eoa_addresses.eth_address), // from
                            H256::from(*ACCOUNT_ADDRESS_EVM)               // to
                        ],
                        data: Bytes::from_str("0x0000000000000000000000000000000000000000000000000000000000002710")
                            .unwrap(), // amount
                        block_hash: logs[1].block_hash, // block hash changes so just set to event value
                        block_number: logs[1].block_number, // block number changes so just set to event value
                        transaction_hash: Some(transfer_tx_hash),
                        transaction_index: None,
                        log_index: None,
                        removed: false
                    },
                    logs[1]
                );
            }
            _ => panic!("Expected FilterChanges::Logs variant, got {:?}", logs),
        }
    }
}
