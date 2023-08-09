mod tests {

    use ctor::ctor;
    use kakarot_rpc_core::client::api::KakarotEthApi;
    use kakarot_rpc_core::models::balance::{TokenBalance, TokenBalances};
    use kakarot_rpc_core::test_utils::deploy_helpers::{
        create_raw_ethereum_tx, KakarotTestEnvironmentContext, TestContext,
    };
    use kakarot_rpc_core::test_utils::fixtures::kakarot_test_env_ctx;
    use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};
    use rstest::*;
    use starknet::core::types::FieldElement;
    use tracing_subscriber::FmtSubscriber;

    #[ctor]
    fn setup() {
        let subscriber = FmtSubscriber::builder().with_max_level(tracing::Level::ERROR).finish();
        tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_rpc_should_not_raise_when_eoa_not_deployed(
        #[with(TestContext::Simple)] kakarot_test_env_ctx: KakarotTestEnvironmentContext,
    ) {
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
    async fn test_eoa_balance(#[with(TestContext::Simple)] kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
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
    async fn test_counter(#[with(TestContext::Counter)] kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot, counter, counter_eth_address) = kakarot_test_env_ctx.resources_with_contract("Counter");

        // When
        let hash = execute_tx(&test_environment, "Counter", "inc", vec![]).await;
        client.transaction_receipt(hash).await.expect("increment transaction failed");

        let count_selector = counter.abi.function("count").unwrap().short_signature();
        let counter_bytes = client
            .call(
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
    async fn test_plain_opcodes(
        #[with(TestContext::PlainOpcodes)] kakarot_test_env_ctx: KakarotTestEnvironmentContext,
    ) {
        // Given
        let (client, _, _, plain_opcodes_eth_address) = kakarot_test_env_ctx.resources_with_contract("PlainOpcodes");
        // Then
        client
            .get_code(plain_opcodes_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .expect("contract not deployed");
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_storage_at(#[with(TestContext::Counter)] kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot, counter, counter_eth_address) = kakarot_test_env_ctx.resources_with_contract("Counter");
        // When
        execute_tx(&test_environment, "Counter", "inc", vec![]).await;

        // Then
        let count = client
            .storage_at(counter_eth_address, U256::from(0), BlockId::Number(BlockNumberOrTag::Latest))
            .await
            .unwrap();
        assert_eq!(U256::from(1), count);
    }

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_token_balances(#[with(TestContext::ERC20)] kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Given
        let (client, kakarot, erc20, erc20_eth_address) = kakarot_test_env_ctx.resources_with_contract("ERC20");

        // When
        let to = U256::try_from_be_slice(&kakarot.eoa_addresses.eth_address.to_fixed_bytes()[..]).unwrap();
        let amount = U256::from(10_000);
        execute_tx(&test_environment, "ERC20", "mint", vec![to, amount]).await;

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

    #[tokio::test]
    async fn test_get_logs_integration() {
        // Given
        let test_environment = KakarotTestEnvironment::new()
            .await
            .deploy_evm_contract(ContractDeploymentArgs {
                name: "ERC20".into(),
                constructor_args: (
                    Token::String("Test".into()),               // name
                    Token::String("TT".into()),                 // symbol
                    Token::Uint(ethers::types::U256::from(18)), // decimals
                ),
            })
            .await;
        let erc20 = test_environment.evm_contract("ERC20");
        let client = test_environment.client();
        let kakarot = test_environment.kakarot();

        // When
        let to = U256::try_from_be_slice(&kakarot.eoa_addresses.eth_address.to_fixed_bytes()[..]).unwrap();
        let amount = U256::from(10_000);
        execute_tx(&test_environment, "ERC20", "mint", vec![to, amount]).await;

        let to = U256::try_from_be_slice(ACCOUNT_ADDRESS_EVM.as_bytes()).unwrap();
        let amount = U256::from(10_000);
        execute_tx(&test_environment, "ERC20", "transfer", vec![to, amount]).await;

        let filter = Filter {
            block_option: FilterBlockOption::Range {
                from_block: Some(BlockNumberOrTag::Number(0)),
                to_block: Some(BlockNumberOrTag::Number(100)),
            },
            address: Some(ValueOrArray::Value(erc20.addresses.eth_address)),
            topics: [None, None, None, None],
        };
        let events = client.get_logs(filter).await.unwrap();

        dbg!(client.kakarot_address());
        dbg!(erc20.addresses.eth_address);

        let provider = client.starknet_provider();
        let filter =
            EventFilter { address: Some(client.kakarot_address()), from_block: None, to_block: None, keys: None };
        let provider_events = provider.get_events(filter, None, 10).await.unwrap();
        dbg!(provider_events);

        // Then
        assert_eq!(2, events.len());
    }
}
