#![cfg(feature = "testing")]
use ethers::{
    abi::Token,
    core::types::{Address as EthersAddress, U256 as EthersU256},
};
use kakarot_rpc::{
    models::{balance::TokenBalances, felt::Felt252Wrapper},
    test_utils::{
        eoa::Eoa as _,
        evm_contract::KakarotEvmContract,
        fixtures::{erc20, setup},
        katana::Katana,
        rpc::start_kakarot_rpc_server,
    },
};
use reth_primitives::{Address, U256};
use rstest::*;
use serde_json::{json, Value};

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_token_balances(#[future] erc20: (Katana, KakarotEvmContract), _setup: ()) {
    // Given
    let katana = erc20.0;
    let erc20 = erc20.1;
    let eoa = katana.eoa();
    let eoa_address = eoa.evm_address().expect("Failed to get Eoa EVM address");
    let erc20_address: Address =
        Into::<Felt252Wrapper>::into(erc20.evm_address).try_into().expect("Failed to convert EVM address");

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // When
    let to = EthersAddress::from_slice(eoa.evm_address().unwrap().as_slice());
    let amount = U256::from(10_000);

    eoa.call_evm_contract(
        &erc20,
        "mint",
        (Token::Address(to), Token::Uint(EthersU256::from_big_endian(&amount.to_be_bytes::<32>()[..]))),
        0,
    )
    .await
    .expect("Failed to mint ERC20 tokens");

    // Then
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            json!(
                {
                    "jsonrpc":"2.0",
                    "method":"alchemy_getTokenBalances",
                    "params":[eoa_address, [erc20_address]],
                    "id":1,
                }
            )
            .to_string(),
        )
        .send()
        .await
        .expect("Failed to call Alchemy RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let balances: TokenBalances =
        serde_json::from_value(raw.get("result").cloned().unwrap()).expect("Failed to deserialize response body");
    let erc20_balance = balances.token_balances[0].token_balance.expect("Failed to get ERC20 balance");

    assert_eq!(amount, erc20_balance);
    drop(server_handle);
}
