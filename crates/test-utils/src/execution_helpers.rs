use std::sync::Arc;

use ethers::signers::{LocalWallet, Signer};
use kakarot_rpc_core::client::api::KakarotEthApi;
use kakarot_rpc_core::client::waiter::TransactionWaiter;
use reth_primitives::{Address, BlockId, H256, U256};
use starknet::accounts::{Account, Call, ConnectedAccount, SingleOwnerAccount};
use starknet::core::types::InvokeTransactionResult;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet::signers::LocalWallet as StarknetLocalWallet;

use super::deploy_helpers::{create_eth_transfer_tx, create_raw_ethereum_tx, KakarotTestEnvironmentContext};

pub async fn execute_and_wait_for_tx(
    account: &SingleOwnerAccount<JsonRpcClient<HttpTransport>, StarknetLocalWallet>,
    calls: Vec<Call>,
) -> InvokeTransactionResult {
    let res =
        account.execute(calls.clone()).send().await.unwrap_or_else(|_| panic!("Failed to execute tx: {:?}", calls));

    let waiter = TransactionWaiter::new(Arc::new(account.provider()), res.transaction_hash, 1000, 15_000);
    waiter.poll().await.expect("Failed to poll tx");
    res
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

    client.send_transaction(tx).await.unwrap()
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

    client.send_transaction(tx).await.unwrap()
}
