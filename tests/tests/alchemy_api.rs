#![allow(clippy::used_underscore_binding)]
#![cfg(feature = "testing")]
use std::str::FromStr;

use alloy_dyn_abi::DynSolValue;
use kakarot_rpc::{
    models::{
        felt::Felt252Wrapper,
        token::{TokenBalances, TokenMetadata},
    },
    test_utils::{
        eoa::Eoa as _,
        evm_contract::KakarotEvmContract,
        fixtures::{erc20, setup},
        katana::Katana,
        rpc::{start_kakarot_rpc_server, RawRpcParamsBuilder},
    },
};
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
    // Get the recipient address for minting tokens
    let to = Address::from_slice(eoa.evm_address().unwrap().as_slice());
    // Set the amount of tokens to mint
    let amount = U256::from(10_000);
    // Call the mint function of the ERC20 contract
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
    // Get the response body
    let response = res.text().await.expect("Failed to get response body");

    // Deserialize the response body
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");

    // Deserialize the token balances from the response
    let balances: TokenBalances =
        serde_json::from_value(raw.get("result").cloned().unwrap()).expect("Failed to deserialize response body");

    // Get the ERC20 balance from the token balances
    let erc20_balance = balances.token_balances[0].token_balance;

    // Assert that the ERC20 balance matches the minted amount
    assert_eq!(amount, erc20_balance);

    // Clean up by dropping the Kakarot RPC server handle
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_token_metadata(#[future] erc20: (Katana, KakarotEvmContract), _setup: ()) {
    // Obtain the Katana instance
    let katana = erc20.0;

    // Obtain the ERC20 contract instance
    let erc20 = erc20.1;

    // Convert the ERC20 EVM address
    let erc20_address: Address =
        Felt252Wrapper::from(erc20.evm_address).try_into().expect("Failed to convert EVM address");

    // Start the Kakarot RPC server
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // When
    // Construct and send RPC request for token metadata
    let reqwest_client = reqwest::Client::new();
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(RawRpcParamsBuilder::new("alchemy_getTokenMetadata").add_param(erc20_address).build())
        .send()
        .await
        .expect("Failed to call Alchemy RPC");

    // Then
    // Verify the response
    let response = res.text().await.expect("Failed to get response body");

    // Deserialize the response body
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");

    // Deserialize the token metadata from the response
    let metadata: TokenMetadata =
        serde_json::from_value(raw.get("result").cloned().unwrap()).expect("Failed to deserialize response body");

    // Assert that the token metadata fields match the expected values
    assert_eq!(metadata.decimals, U256::from(18));
    assert_eq!(metadata.name, "Test");
    assert_eq!(metadata.symbol, "TT");

    // Clean up by dropping the Kakarot RPC server handle
    drop(server_handle);
}

#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_token_allowance(#[future] erc20: (Katana, KakarotEvmContract), _setup: ()) {
    // Obtain the Katana instance
    let katana = erc20.0;

    // Obtain the ERC20 contract instance
    let erc20 = erc20.1;

    // Get the EOA (Externally Owned Account)
    let eoa = katana.eoa();

    // Get the EVM address of the EOA
    let eoa_address = eoa.evm_address().expect("Failed to get Eoa EVM address");

    // Convert the ERC20 EVM address
    let erc20_address: Address =
        Felt252Wrapper::from(erc20.evm_address).try_into().expect("Failed to convert EVM address");

    // Set the spender address for testing allowance
    let spender_address = Address::from_str("0x1234567890123456789012345678901234567890").expect("Invalid address");

    // Start the Kakarot RPC server
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    // When
    // Set the allowance amount
    let allowance_amount = U256::from(5000);

    // Call the approve function of the ERC20 contract
    eoa.call_evm_contract(
        &erc20,
        "approve",
        &[DynSolValue::Address(spender_address), DynSolValue::Uint(allowance_amount, 256)],
        0,
    )
    .await
    .expect("Failed to approve allowance for ERC20 tokens");

    // Then
    let reqwest_client = reqwest::Client::new();

    // Send a POST request to the Kakarot RPC server
    let res = reqwest_client
        .post(format!("http://localhost:{}", server_addr.port()))
        .header("Content-Type", "application/json")
        .body(
            RawRpcParamsBuilder::new("alchemy_getTokenAllowance")
                .add_param(erc20_address)
                .add_param(eoa_address)
                .add_param(spender_address)
                .build(),
        )
        .send()
        .await
        .expect("Failed to call Alchemy RPC");

    // Get the response body
    let response = res.text().await.expect("Failed to get response body");

    // Deserialize the response body
    let raw: Value = serde_json::from_str(&response).expect("Failed to deserialize response body");

    // Deserialize the allowance amount from the response
    let allowance: U256 =
        serde_json::from_value(raw.get("result").cloned().unwrap()).expect("Failed to deserialize response body");

    // Assert that the allowance amount matches the expected amount
    assert_eq!(allowance, allowance_amount);

    // Clean up by dropping the Kakarot RPC server handle
    drop(server_handle);
}
