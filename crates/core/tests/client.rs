mod tests {

    use ctor::ctor;
    use ethers::abi::Token;
    use ethers::types::Address as EthersAddress;
    use kakarot_rpc_core::client::api::KakarotEthApi;
    use kakarot_rpc_core::models::balance::{TokenBalance, TokenBalances};
    use kakarot_rpc_core::models::felt::Felt252Wrapper;
    use kakarot_rpc_core::test_utils::deploy_helpers::{
        create_raw_ethereum_tx, ContractDeploymentArgs, KakarotTestEnvironment,
    };
    use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};
    use starknet::core::types::FieldElement;
    use tracing_subscriber::FmtSubscriber;

    #[ctor]
    fn setup() {
        let subscriber = FmtSubscriber::builder().with_max_level(tracing::Level::ERROR).finish();
        tracing::subscriber::set_global_default(subscriber).expect("setting tracing default failed");
    }

    #[tokio::test]
    async fn test_rpc_should_not_raise_when_eoa_not_deployed() {
        // Given
        let test_environment = KakarotTestEnvironment::new().await;

        // When
        let nonce =
            test_environment.client().nonce(Address::zero(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

        // Then
        // Zero address shouldn't throw 'ContractNotFound', but return zero
        assert_eq!(U256::from(0), nonce);
    }

    #[tokio::test]
    async fn test_eoa_balance() {
        // Given
        let test_environment = KakarotTestEnvironment::new().await;
        let client = test_environment.client();
        let kakarot = test_environment.kakarot();

        // When
        let eoa_balance = client
            .balance(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let eoa_balance = FieldElement::from_bytes_be(&eoa_balance.to_be_bytes()).unwrap();

        // Then
        assert_eq!(FieldElement::from_dec_str("1000000000000000000").unwrap(), eoa_balance);
    }

    #[tokio::test]
    async fn test_counter() {
        // Given
        let test_environment = KakarotTestEnvironment::new()
            .await
            .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
            .await;
        let client = test_environment.client();
        let kakarot = test_environment.kakarot();
        let counter = test_environment.evm_contract("Counter");

        let counter_eth_address = {
            let address: Felt252Wrapper = counter.addresses.eth_address.into();
            address.try_into().unwrap()
        };

        client
            .get_code(counter_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .expect("contract not deployed");

        // When
        let inc_selector = counter.abi.function("inc").unwrap().short_signature();

        let nonce = client
            .nonce(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let inc_tx = create_raw_ethereum_tx(
            inc_selector,
            kakarot.eoa_private_key,
            counter_eth_address,
            vec![],
            nonce.try_into().unwrap(),
        );
        let inc_res = client.send_transaction(inc_tx).await.unwrap();

        client.transaction_receipt(inc_res).await.expect("increment transaction failed");

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

    #[tokio::test]
    async fn test_plain_opcodes() {
        // Given
        let mut test_environment = KakarotTestEnvironment::new().await;

        test_environment = test_environment
            .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
            .await;
        let counter = test_environment.evm_contract("Counter");
        let counter_eth_address: Address = {
            let address: Felt252Wrapper = counter.addresses.eth_address.into();
            address.try_into().unwrap()
        };

        // When
        test_environment = test_environment
            .deploy_evm_contract(ContractDeploymentArgs {
                name: "PlainOpcodes".into(),
                constructor_args: (EthersAddress::from(counter_eth_address.as_fixed_bytes()),),
            })
            .await;
        let plain_opcodes = test_environment.evm_contract("PlainOpcodes");
        let plain_opcodes_eth_address: Address = {
            let address: Felt252Wrapper = plain_opcodes.addresses.eth_address.into();
            address.try_into().unwrap()
        };

        // Then
        let client = test_environment.client();
        client
            .get_code(plain_opcodes_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .expect("contract not deployed");
    }

    #[tokio::test]
    async fn test_storage_at() {
        // Given
        let test_environment = KakarotTestEnvironment::new()
            .await
            .deploy_evm_contract(ContractDeploymentArgs { name: "Counter".into(), constructor_args: () })
            .await;
        let counter = test_environment.evm_contract("Counter");
        let counter_eth_address = {
            let address: Felt252Wrapper = counter.addresses.eth_address.into();
            address.try_into().unwrap()
        };
        let client = test_environment.client();
        let kakarot = test_environment.kakarot();

        // When
        let inc_selector = counter.abi.function("inc").unwrap().short_signature();

        let nonce = client
            .nonce(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();

        let inc_tx = create_raw_ethereum_tx(
            inc_selector,
            kakarot.eoa_private_key,
            counter_eth_address,
            vec![],
            nonce.try_into().unwrap(),
        );

        client.send_transaction(inc_tx).await.unwrap();

        let count = client
            .storage_at(counter_eth_address, U256::from(0), BlockId::Number(BlockNumberOrTag::Latest))
            .await
            .unwrap();

        // Then
        assert_eq!(U256::from(1), count);
    }

    #[tokio::test]
    async fn test_token_balances() {
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
        let erc20_eth_address = {
            let address: Felt252Wrapper = erc20.addresses.eth_address.into();
            address.try_into().unwrap()
        };
        let client = test_environment.client();
        let kakarot = test_environment.kakarot();

        // When
        let nonce = client
            .nonce(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let mint_selector = erc20.abi.function("mint").unwrap().short_signature();

        let to = U256::try_from_be_slice(&kakarot.eoa_addresses.eth_address.to_fixed_bytes()[..]).unwrap();
        let amount = U256::from(10_000);
        let mint_tx = create_raw_ethereum_tx(
            mint_selector,
            kakarot.eoa_private_key,
            erc20_eth_address,
            vec![to, amount],
            nonce.try_into().unwrap(),
        );

        client.send_transaction(mint_tx).await.unwrap();

        let balances = client.token_balances(kakarot.eoa_addresses.eth_address, vec![erc20_eth_address]).await.unwrap();

        // Then
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
}
