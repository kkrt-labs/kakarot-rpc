mod tests {

    use std::str::FromStr;

    use ctor::ctor;
    use ethers::signers::{LocalWallet, Signer};
    use kakarot_rpc_core::client::api::{KakarotEthApi, KakarotStarknetApi};
    use kakarot_rpc_core::client::constants::{DEPLOY_FEE, TX_ORIGIN_ZERO};
    use kakarot_rpc_core::mock::constants::ACCOUNT_ADDRESS_EVM;
    use kakarot_rpc_core::models::balance::{TokenBalance, TokenBalances};
    use kakarot_rpc_core::models::felt::Felt252Wrapper;
    use kakarot_rpc_core::test_utils::constants::EOA_RECEIVER_ADDRESS;
    use kakarot_rpc_core::test_utils::deploy_helpers::KakarotTestEnvironmentContext;
    use kakarot_rpc_core::test_utils::execution_helpers::{execute_eth_transfer_tx, execute_eth_tx};
    use kakarot_rpc_core::test_utils::fixtures::kakarot_test_env_ctx;
    use reth_primitives::{Address, BlockId, BlockNumberOrTag, Bytes, H256, U256};
    use reth_rpc_types::{Filter, FilterBlockOption, FilterChanges, Log, ValueOrArray};
    use rstest::*;
    use starknet::core::types::{
        BlockId as StarknetBlockId, BlockTag, FieldElement, MaybePendingTransactionReceipt, TransactionReceipt,
        TransactionStatus,
    };
    use starknet::providers::Provider;
    use tracing_subscriber::{filter, FmtSubscriber};

    #[ctor]
    fn setup() {
        let filter = filter::EnvFilter::new("info");
        let subscriber = FmtSubscriber::builder().with_env_filter(filter).finish();
        tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_rpc_should_not_raise_when_eoa_not_deployed(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let client = kakarot_test_env_ctx.client();

        // When
        let nonce = client.nonce(Address::zero(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

        // Then
        // Zero address shouldn't throw 'ContractNotFound', but return zero
        assert_eq!(U256::from(0), nonce);
    }

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
    async fn test_counter(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, _, counter, counter_eth_address) = kakarot_test_env_ctx.resources_with_contract("Counter");

        // When
        let hash = execute_eth_tx(&kakarot_test_env_ctx, "Counter", "inc", vec![]).await;
        client.transaction_receipt(hash).await.expect("increment transaction failed");

        let count_selector = counter.abi.function("count").unwrap().short_signature();
        let counter_bytes = client
            .call(
                *TX_ORIGIN_ZERO,
                counter_eth_address,
                count_selector.into(),
                BlockId::Number(reth_primitives::BlockNumberOrTag::Latest),
            )
            .await
            .unwrap();

        let num = *counter_bytes.last().expect("Empty byte array");

        // Then
        assert_eq!(num, 1);
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

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_wait_for_confirmation_on_l2(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        let (client, kakarot) = kakarot_test_env_ctx.resources();
        let amount = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

        let transaction_hash =
            execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, *EOA_RECEIVER_ADDRESS, amount)
                .await;
        let transaction_hash: FieldElement = Felt252Wrapper::try_from(transaction_hash).unwrap().into();

        let _ = client.wait_for_confirmation_on_l2(transaction_hash).await;

        let transaction_receipt = client.starknet_provider().get_transaction_receipt(transaction_hash).await.unwrap();

        match transaction_receipt {
            MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke(receipt)) => {
                assert_eq!(TransactionStatus::AcceptedOnL2, receipt.status)
            }
            _ => panic!(
                "Expected MaybePendingTransactionReceipt::Receipt(TransactionReceipt::Invoke), got {:?}",
                transaction_receipt
            ),
        }
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_check_eoa_account_exists(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        let (client, kakarot) = kakarot_test_env_ctx.resources();
        let block_id = StarknetBlockId::Tag(BlockTag::Latest);
        // this address shouldn't be shared with other tests, otherwise a test might deploy it in parallel,
        // and this test will fail; source -> ganache (https://github.com/trufflesuite/ganache)
        let evm_address_not_existing = Address::from_str("0xcE16e8eb8F4BF2E65BA9536C07E305b912BAFaCF").unwrap();

        let res = client.check_eoa_account_exists(kakarot.eoa_addresses.eth_address, &block_id).await.unwrap();
        assert!(res);

        let res = client.check_eoa_account_exists(evm_address_not_existing, &block_id).await.unwrap();
        assert!(!res)
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_deploy_eoa(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        let (client, kakarot) = kakarot_test_env_ctx.resources();
        let block_id = StarknetBlockId::Tag(BlockTag::Latest);
        // this address shouldn't be shared with other tests, otherwise a test might deploy it in parallel,
        // and this test will fail; source -> ganache (https://github.com/trufflesuite/ganache)
        let ethereum_address_to_deploy = Address::from_str("0x02f1c4C93AFEd946Cce5Ad7D34354A150bEfCFcF").unwrap();
        let amount: u128 = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

        // checking the account is not already deployed
        let res = client.check_eoa_account_exists(ethereum_address_to_deploy, &block_id).await.unwrap();
        assert!(!res);

        // funding account so it can cover its deployment fee
        let _ =
            execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, ethereum_address_to_deploy, amount)
                .await;

        let _ = client.deploy_eoa(ethereum_address_to_deploy).await.unwrap();

        // checking that the account is deployed
        let res = client.check_eoa_account_exists(ethereum_address_to_deploy, &block_id).await.unwrap();
        assert!(res);

        let balance =
            client.balance(ethereum_address_to_deploy, BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();
        assert_eq!(balance, U256::ZERO);
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_automatic_deployment_of_eoa(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        let (_, kakarot) = kakarot_test_env_ctx.resources();
        let block_id_latest = BlockId::Number(BlockNumberOrTag::Latest);

        // the private key has been taken from the ganache repo and can be safely published, do no share
        // with other tests https://github.com/trufflesuite/ganache
        let ethereum_private_key = "0x7f109a9e3b0d8ecfba9cc23a3614433ce0fa7ddcc80f2a8f10b222179a5a80d6";
        let to = LocalWallet::from_str(ethereum_private_key).unwrap();
        let to_private_key = {
            let signing_key_bytes = to.signer().to_bytes(); // Convert to bytes
            H256::from_slice(&signing_key_bytes) // Convert to H256
        };
        let to_address: Address = to.address().into();

        let deploy_fee: u128 = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

        let _ =
            execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, to_address, deploy_fee * 2).await;

        let balance = kakarot_test_env_ctx.client().balance(to_address, block_id_latest).await.unwrap();

        assert_eq!(balance, U256::from(deploy_fee * 2));

        let _ = execute_eth_transfer_tx(
            &kakarot_test_env_ctx,
            to_private_key,
            kakarot.eoa_addresses.eth_address,
            deploy_fee,
        )
        .await;

        let balance = kakarot_test_env_ctx.client().balance(to_address, block_id_latest).await.unwrap();

        assert_eq!(balance, U256::ZERO);
    }
}
