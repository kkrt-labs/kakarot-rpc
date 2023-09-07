use std::thread;

use ethers::signers::{LocalWallet, Signer};
use reth_primitives::{Address, BlockId, H256, U256};
use starknet::accounts::{Account, Call, SingleOwnerAccount};
use starknet::core::types::InvokeTransactionResult;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet::signers::LocalWallet as StarknetLocalWallet;
use tokio::time;

use super::deploy_helpers::{create_eth_transfer_tx, create_raw_ethereum_tx, KakarotTestEnvironmentContext};
use crate::client::api::KakarotEthApi;

pub async fn execute_tx(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, StarknetLocalWallet>,
    calls: Vec<Call>,
) -> InvokeTransactionResult {
    let c = calls.clone();
    let res = account.execute(calls).send().await.expect(format!("Failed to execute tx: {:?}", c).as_str());

    wait_for_tx();
    res
}

pub fn wait_for_tx() {
    thread::sleep(time::Duration::from_secs(15));
}

pub async fn execute_eth_tx(
    env: &KakarotTestEnvironmentContext,
    contract: &str,
    selector: &str,
    args: Vec<U256>,
) -> H256 {
    let (client, kakarot, contract, contract_eth_address) = env.resources_with_contract(contract);

    // When
    let nonce = client
        .nonce(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
        .await
        .unwrap();
    let selector = contract.abi.function(selector).unwrap().short_signature();

    let tx = create_raw_ethereum_tx(
        selector,
        kakarot.eoa_private_key,
        contract_eth_address,
        args,
        nonce.try_into().unwrap(),
    );

    let res = client.send_transaction(tx).await.unwrap();
    wait_for_tx();
    res
}

pub async fn execute_eth_transfer_tx(
    env: &KakarotTestEnvironmentContext,
    eoa_secret_key: H256,
    to: Address,
    value: u128,
) -> H256 {
    let (client, _) = env.resources();

    let eoa = LocalWallet::from_bytes(eoa_secret_key.as_bytes()).unwrap();
    let eoa_address: Address = eoa.address().into();

    // When
    let nonce = client.nonce(eoa_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest)).await.unwrap();

    let tx = create_eth_transfer_tx(eoa_secret_key, to, value, nonce.try_into().unwrap());

    let res = client.send_transaction(tx).await.unwrap();
    wait_for_tx();
    res
}
