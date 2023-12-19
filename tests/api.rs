mod test_utils;

use ethers::abi::Token;
use kakarot_rpc::models::felt::Felt252Wrapper;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, U256};
use rstest::*;
use starknet::core::types::FieldElement;
use test_utils::eoa::Eoa;
use test_utils::evm_contract::KakarotEvmContract;
use test_utils::fixtures::{counter, erc20, katana};
use test_utils::sequencer::Katana;

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
    let nonce_initial =
        client.nonce(counter_evm_address.try_into().unwrap(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

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
    let eoa_evm_address = eoa.evm_address().expect("Failed to get Eoa EVM address");
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
    let count =
        client.storage_at(counter_evm_address, U256::from(0), BlockId::Number(BlockNumberOrTag::Latest)).await.unwrap();
    assert_eq!(U256::from(1), count);
}
