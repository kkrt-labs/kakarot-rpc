use std::sync::Arc;

use ethers::abi::Token;
use kakarot_rpc_core::client::waiter::TransactionWaiter;
use reth_primitives::{Address, H256};
use starknet::accounts::{Account, Call, ConnectedAccount, SingleOwnerAccount};
use starknet::core::types::InvokeTransactionResult;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;
use starknet::signers::LocalWallet as StarknetLocalWallet;

use super::deploy_helpers::KakarotTestEnvironmentContext;

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
    _env: &KakarotTestEnvironmentContext,
    _contract: &str,
    _selector: &str,
    _args: Vec<Token>,
) -> H256 {
    todo!();
}

pub async fn execute_eth_transfer_tx(
    _env: &KakarotTestEnvironmentContext,
    _eoa_secret_key: H256,
    _to: Address,
    _value: u128,
) -> H256 {
    todo!();
}
