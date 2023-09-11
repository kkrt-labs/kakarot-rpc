use reth_primitives::U256;
use starknet::core::types::BlockId;
use starknet::providers::SequencerGatewayProvider;
use starknet_crypto::FieldElement;

use crate::client::api::KakarotStarknetApi;
use crate::client::constants::{DUMMY_ARGENT_GAS_PRICE_ACCOUNT_ADDRESS, STARKNET_NATIVE_TOKEN};
use crate::contracts::erc20::starknet_erc20::StarknetErc20;
use crate::mock::mock_starknet::init_testnet_client;

#[tokio::test]
async fn test_balance_of() {
    // Given
    let client = init_testnet_client();
    let starknet_native_token_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap();
    let provider = client.starknet_provider();
    let eth = StarknetErc20::<SequencerGatewayProvider>::new(&provider, starknet_native_token_address);

    let random_block = BlockId::Number(838054);

    // When
    let balance = eth.balance_of(&DUMMY_ARGENT_GAS_PRICE_ACCOUNT_ADDRESS, &random_block).await.unwrap();

    // Then
    assert_eq!(U256::from(983627765290549u64), balance);
}
