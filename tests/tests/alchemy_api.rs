#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use alloy_dyn_abi::DynSolValue;
use kakarot_rpc::models::balance::TokenBalances;
use kakarot_rpc::models::felt::Felt252Wrapper;
use kakarot_rpc::test_utils::eoa::Eoa as _;
use kakarot_rpc::test_utils::evm_contract::KakarotEvmContract;
use kakarot_rpc::test_utils::fixtures::{erc20, setup};
use kakarot_rpc::test_utils::katana::Katana;
use kakarot_rpc::test_utils::rpc::start_kakarot_rpc_server;
use kakarot_rpc::test_utils::rpc::RawRpcParamsBuilder;
use reth_primitives::{Address, U256};
use rstest::*;
use serde_json::Value;

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
        Felt252Wrapper::from(erc20.evm_address).try_into().expect("Failed to convert EVM address");

    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // When
    let to = Address::from_slice(eoa.evm_address().unwrap().as_slice());
    let amount = U256::from(10_000);

    eoa.call_evm_contract(&erc20, "mint", &[DynSolValue::Address(to), DynSolValue::Uint(amount, 256)], 0)
        .await
        .expect("Failed to mint ERC20 tokens");

    // Then
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            RawRpcParamsBuilder::new("alchemy_getTokenBalances")
                .add_param(eoa_address)
                .add_param([erc20_address])
                .build(),
        )
        .send()
        .await
        .expect("Failed to call Alchemy RPC");
    let response = res.text().await.expect("Failed to get response body");
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");
    let balances: TokenBalances =
        serde_json::from_value(raw.get("result").cloned().unwrap()).expect("Failed to deserialize response body");
    let erc20_balance = balances.token_balances[0].token_balance;

    assert_eq!(amount, erc20_balance);
    drop(server_handle);
}
