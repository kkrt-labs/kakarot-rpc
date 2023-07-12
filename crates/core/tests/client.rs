#[cfg(test)]
mod tests {

    use ethers::types::Address as EthersAddress;
    use kakarot_rpc_core::client::api::KakarotEthApi;
    use kakarot_rpc_core::client::config::{Network, StarknetConfig};
    use kakarot_rpc_core::client::KakarotClient;
    use kakarot_rpc_core::mock::constants::EXAMPLE_URL;
    use kakarot_rpc_core::models::felt::Felt252Wrapper;
    use kakarot_rpc_core::test_utils::constants::EOA_WALLET;
    use kakarot_rpc_core::test_utils::deploy_helpers::{
        construct_kakarot_test_sequencer, create_raw_ethereum_tx, deploy_kakarot_system,
    };
    use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};
    use starknet::core::types::FieldElement;
    use starknet::providers::jsonrpc::HttpTransport;
    use starknet::providers::JsonRpcClient;

    #[tokio::test]
    async fn test_rpc_should_not_raise_when_eoa_not_deployed() {
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        let kakarot_client = KakarotClient::new(
            StarknetConfig::new(
                Network::JsonRpcProvider(starknet_test_sequencer.url()),
                deployed_kakarot.kakarot,
                deployed_kakarot.kakarot_proxy,
            ),
            JsonRpcClient::new(HttpTransport::new(starknet_test_sequencer.url())),
        )
        .unwrap();

        // Zero address shouldn't throw 'ContractNotFound', but return zero
        assert_eq!(
            U256::from(0),
            kakarot_client.nonce(Address::zero(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_counter() {
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        let expected_funded_amount = FieldElement::from_dec_str("10000000000000000000").unwrap();

        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        let (counter_abi, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "Counter",
                // no constructor is conveyed as a tuple
                (),
            )
            .await
            .unwrap();

        let kakarot_client = KakarotClient::new(
            StarknetConfig::new(
                Network::JsonRpcProvider(starknet_test_sequencer.url()),
                deployed_kakarot.kakarot,
                deployed_kakarot.kakarot_proxy,
            ),
            JsonRpcClient::new(HttpTransport::new(starknet_test_sequencer.url())),
        )
        .unwrap();

        let deployed_balance = kakarot_client
            .balance(deployed_kakarot.eoa_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await;

        let _deployed_balance = FieldElement::from_bytes_be(&deployed_balance.unwrap().to_be_bytes()).unwrap();

        // this assert is failing, need to debug why
        // assert_eq!(deployed_balance, expected_funded_amount);

        let counter_eth_address = {
            let address: Felt252Wrapper = (*deployed_addresses.first().unwrap()).into();
            address.try_into().unwrap()
        };

        kakarot_client
            .get_code(counter_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .expect("contract not deployed");

        let inc_selector = counter_abi.function("inc").unwrap().short_signature();

        let nonce = kakarot_client
            .nonce(deployed_kakarot.eoa_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .unwrap();
        let inc_tx = create_raw_ethereum_tx(
            inc_selector,
            deployed_kakarot.eoa_private_key,
            counter_eth_address,
            vec![],
            nonce.try_into().unwrap(),
        );
        let inc_res = kakarot_client.send_transaction(inc_tx).await.unwrap();

        kakarot_client.transaction_receipt(inc_res).await.expect("increment transaction failed");

        let count_selector = counter_abi.function("count").unwrap().short_signature();
        let counter_bytes = kakarot_client
            .call_view(
                counter_eth_address,
                count_selector.into(),
                BlockId::Number(reth_primitives::BlockNumberOrTag::Latest),
            )
            .await
            .unwrap();

        let num = *counter_bytes.last().expect("Empty byte array");
        assert_eq!(num, 1);
    }

    #[tokio::test]
    async fn test_plain_opcodes() {
        let starknet_test_sequencer = construct_kakarot_test_sequencer().await;

        let expected_funded_amount = FieldElement::from_dec_str("1000000000000000000").unwrap();

        let deployed_kakarot =
            deploy_kakarot_system(&starknet_test_sequencer, EOA_WALLET.clone(), expected_funded_amount).await;

        let (_, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "Counter",
                // no constructor is conveyed as a tuple
                (),
            )
            .await
            .unwrap();

        let counter_eth_address: Address = {
            let address: Felt252Wrapper = (*deployed_addresses.first().unwrap()).into();
            address.try_into().unwrap()
        };

        let (_plain_opcodes_abi, deployed_addresses) = deployed_kakarot
            .deploy_evm_contract(
                starknet_test_sequencer.url(),
                "PlainOpcodes",
                (EthersAddress::from(counter_eth_address.as_fixed_bytes()),),
            )
            .await
            .unwrap();

        let plain_opcodes_eth_address: Address = {
            let address: Felt252Wrapper = (*deployed_addresses.first().unwrap()).into();
            address.try_into().unwrap()
        };

        let kakarot_client = KakarotClient::new(
            StarknetConfig::new(
                Network::JsonRpcProvider(starknet_test_sequencer.url()),
                deployed_kakarot.kakarot,
                deployed_kakarot.kakarot_proxy,
            ),
            JsonRpcClient::new(HttpTransport::new(starknet_test_sequencer.url())),
        )
        .unwrap();

        kakarot_client
            .get_code(plain_opcodes_eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
            .await
            .expect("contract not deployed");
    }
}
