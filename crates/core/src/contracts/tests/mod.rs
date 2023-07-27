use reth_primitives::U256;
use starknet::core::types::BlockId;
use starknet::providers::SequencerGatewayProvider;
use starknet_crypto::FieldElement;

use crate::client::api::KakarotStarknetApi;
use crate::client::constants::STARKNET_NATIVE_TOKEN;
use crate::client::tests::init_testnet_client;
use crate::contracts::erc20::Erc20Contract;

#[tokio::test]
async fn test_balance_of() {
    // Given
    let client = init_testnet_client();
    let starknet_native_token_address = FieldElement::from_hex_be(STARKNET_NATIVE_TOKEN).unwrap();
    let erc20 = Erc20Contract::<SequencerGatewayProvider>::new(starknet_native_token_address);

    let target_address =
        FieldElement::from_hex_be("0x05590dc5e5bddf4b334e31713f8caf820f58f6393189e33aeae891ba8534aeb6").unwrap();
    let target_block = BlockId::Number(838054);

    // When
    let balance = erc20.balance_of(client.starknet_provider(), &target_address, &target_block).await.unwrap();

    // Then
    assert_eq!(U256::from(35939073425845666u64), balance);
}
