mod tests {

    use std::str::FromStr;

    use ethers::abi::Token;
    use kakarot_rpc_core::client::api::KakarotEthApi;
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS_EVM;
    use kakarot_rpc_core::models::felt::Felt252Wrapper;
    use kakarot_test_utils::execution::contract::KakarotEvmContract;
    use kakarot_test_utils::execution::eoa::EOA;
    use kakarot_test_utils::fixtures::{counter, erc20, katana};
    use kakarot_test_utils::sequencer::Katana;
    use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, H256, U256};
    use reth_rpc_types::{Filter, FilterBlockOption, FilterChanges, Log, ValueOrArray};
    use rstest::*;
    use starknet::core::types::FieldElement;

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_nonce_eoa(#[future] katana: Katana) {
        // Given
        let client = katana.client();

        // When
        let nonce = client.nonce(Address::zero(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

        // Then
        // Zero address shouldn't throw 'ContractNotFound', but return zero
        assert_eq!(U256::from(0), nonce);
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_nonce_contract_account(#[future] counter: (Katana, KakarotEvmContract)) {
        // Given
        let katana = counter.0;
        let counter = counter.1;
        let client = katana.client();
        let counter_evm_address: Felt252Wrapper = counter.evm_address.into();

        // When
        let nonce_initial = client
            .nonce(counter_evm_address.try_into().unwrap(), BlockId::from(BlockNumberOrTag::Latest))
            .await
            .unwrap();

        // Then
        assert_eq!(nonce_initial, U256::from(1));
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_eoa_balance(#[future] katana: Katana) {
        // Given
        let client = katana.client();
        let eoa = katana.eoa();

        // When
        let eoa_balance = client
            .balance(eoa.evm_address().unwrap(), BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let eoa_balance = FieldElement::from_bytes_be(&eoa_balance.to_be_bytes()).unwrap();

        // Then
        assert!(eoa_balance > FieldElement::ZERO);
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_token_balances(#[future] erc20: (Katana, KakarotEvmContract)) {
        // Given
        let katana = erc20.0;
        let erc20 = erc20.1;
        let client = katana.client();
        let eoa = katana.eoa();
        let eoa_evm_address = eoa.evm_address().expect("Failed to get EOA EVM address");
        let erc20_evm_address: Felt252Wrapper = erc20.evm_address.into();
        let erc20_evm_address = erc20_evm_address.try_into().expect("Failed to convert EVM address");

        // When
        let to = eoa.evm_address().unwrap();
        let amount = U256::from(10_000);
        eoa.call_evm_contract(&erc20, "mint", (Token::Address(to.into()), Token::Uint(amount.into())), 0)
            .await
            .expect("Failed to mint ERC20 tokens");

        // Then
        let balances = client.token_balances(eoa_evm_address, vec![erc20_evm_address]).await.unwrap();
        let erc20_balance = balances.token_balances[0].token_balance.expect("Failed to get ERC20 balance");

        assert_eq!(amount, erc20_balance);
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_storage_at(#[future] counter: (Katana, KakarotEvmContract)) {
        // Given
        let katana = counter.0;
        let counter = counter.1;
        let client = katana.client();
        let eoa = katana.eoa();
        let counter_evm_address: Felt252Wrapper = counter.evm_address.into();
        let counter_evm_address = counter_evm_address.try_into().expect("Failed to convert EVM address");

        // When
        eoa.call_evm_contract(&counter, "inc", (), 0).await.expect("Failed to increment counter");

        // Then
        let count = client
            .storage_at(counter_evm_address, U256::from(0), BlockId::Number(BlockNumberOrTag::Latest))
            .await
            .unwrap();
        assert_eq!(U256::from(1), count);
    }

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_logs(#[future] erc20: (Katana, KakarotEvmContract)) {
        // Given
        let katana = erc20.0;
        let erc20 = erc20.1;
        let client = katana.client();
        let eoa = katana.eoa();
        let eoa_evm_address = eoa.evm_address().expect("Failed to get EOA EVM address");
        let erc20_evm_address: Felt252Wrapper = erc20.evm_address.into();
        let erc20_evm_address = erc20_evm_address.try_into().expect("Failed to convert EVM address");

        // When
        let amount = U256::from(10_000);

        let mint_tx_hash = eoa
            .call_evm_contract(&erc20, "mint", (Token::Address(eoa_evm_address.into()), Token::Uint(amount.into())), 0)
            .await
            .expect("Failed to mint ERC20 tokens");

        let to = Address::from_slice(ACCOUNT_ADDRESS_EVM.as_bytes());
        let transfer_tx_hash = eoa
            .call_evm_contract(&erc20, "transfer", (Token::Address(to.into()), Token::Uint(amount.into())), 0)
            .await
            .expect("Failed to transfer ERC20 tokens");

        let filter = Filter {
            block_option: FilterBlockOption::Range {
                from_block: Some(BlockNumberOrTag::Number(0)),
                to_block: Some(BlockNumberOrTag::Number(100)),
            },
            address: ValueOrArray::Value(erc20_evm_address).into(),
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
                        address: erc20_evm_address,
                        topics: vec![
                            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                                .unwrap(), // keccak256(“Transfer(address,address,uint256)”)
                            H256::from_low_u64_be(0u64), // from
                            H256::from(eoa_evm_address)  // to
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
                        address: erc20_evm_address,
                        topics: vec![
                            H256::from_str("0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef")
                                .unwrap(), // keccak256("Transfer(address,address,uint256)")
                            H256::from(eoa_evm_address),      // from
                            H256::from(*ACCOUNT_ADDRESS_EVM)  // to
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
