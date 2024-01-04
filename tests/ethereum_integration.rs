mod test_utils;
use std::convert::TryFrom;
use std::sync::Arc;
use std::time::Duration;

use ethers::contract::ContractFactory;
use ethers::core::k256::ecdsa::SigningKey;
use ethers::middleware::SignerMiddleware;
use ethers::prelude::abigen;
use ethers::providers::{Http, Middleware, Provider};
use ethers::signers::{LocalWallet, Signer};
use ethers::types::{BlockId, BlockNumber, TransactionReceipt, H160, H256};
use ethers::utils::keccak256;
use hex::FromHex;
use reth_primitives::{U256, U64};
use rstest::*;
use test_utils::eoa::Eoa;
use test_utils::fixtures::katana;
use test_utils::rpc::start_kakarot_rpc_server;
use test_utils::sequencer::Katana;

abigen!(ERC20, "tests/ERC20/IERC20.json");

// ⚠️ Only one test with `start_kakarot_rpc_server`
// When trying to run two tests with a server originating from `start_kakarot_rpc_server`, the
// second test will fail with: `thread 'test_erc20' panicked at 'Failed to start the server: Os
// { code: 98, kind: AddrInUse, message: "Address already in use" }'`
#[rstest]
#[awt]
#[tokio::test(flavor = "multi_thread")]
async fn test_erc20(#[future] katana: Katana) {
    let (server_addr, server_handle) =
        start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

    let reqwest_client = reqwest::Client::new();
    let _res = reqwest_client
        .post(format!("http://localhost:{}/net_health", server_addr.port()))
        .send()
        .await
        .expect("net_health: health check failed");

    let wallet: LocalWallet = SigningKey::from_slice(katana.eoa().private_key().as_ref())
        .expect("Eoa Private Key should be used to init a LocalWallet")
        .into();

    let provider = Provider::<Http>::try_from(format!("http://localhost:{}", server_addr.port()))
        .unwrap()
        .interval(Duration::from_millis(10u64));

    // get_chainid() returns a U256, which is a [u64; 4]
    // We only need the first u64
    let chain_id = provider.get_chainid().await.unwrap().0[0];
    let client = Arc::new(SignerMiddleware::new(provider, wallet.with_chain_id(chain_id)));

    let block_number: U64 = client.get_block_number().await.unwrap();
    let params = BlockId::Number(BlockNumber::Number(block_number));
    let block = client.get_block(params).await;
    assert!(block.is_ok());

    let bytecode = include_str!("ERC20/bytecode.json");
    let bytecode: serde_json::Value = serde_json::from_str(bytecode).unwrap();
    // Deploy an ERC20
    let factory = ContractFactory::new(
        ERC20_ABI.clone(),
        ethers::types::Bytes::from_hex(bytecode["bytecode"].as_str().unwrap()).unwrap(),
        client.clone(),
    );

    let contract = factory.deploy(()).unwrap().send().await.unwrap();
    let token = ERC20::new(contract.address(), client.clone());

    // Assert initial balance is 0
    let balance = token
        .balance_of(katana.eoa().evm_address().unwrap().into())
        .gas(U256::from(0xffffffffffffffffffffffffffffffff_u128))
        .call()
        .await
        .unwrap();
    assert_eq!(balance, 0u64.into());

    // Mint some tokens
    let tx_receipt: TransactionReceipt = token.mint(100u64.into()).send().await.unwrap().await.unwrap().unwrap();
    let block_number: U64 = client.get_block_number().await.unwrap();

    // Assert balance is now 100
    let balance = token
        .balance_of(katana.eoa().evm_address().unwrap().into())
        .gas(U256::from(0xffffffffffffffffffffffffffffffff_u128))
        .call()
        .await
        .unwrap();
    assert_eq!(balance, 100u64.into());

    // Assert on the transaction receipt
    assert_eq!(tx_receipt.status, Some(1u64.into()));
    assert_eq!(tx_receipt.transaction_index, 0.into());
    assert_eq!(tx_receipt.block_number, Some(block_number));
    assert_eq!(tx_receipt.from, katana.eoa().evm_address().unwrap().into());
    assert_eq!(tx_receipt.to, Some(contract.address()));
    // Assert on the logs
    assert_eq!(tx_receipt.logs.len(), 1);
    assert_eq!(tx_receipt.logs[0].topics.len(), 3);
    assert_eq!(tx_receipt.logs[0].topics[0], H256::from_slice(&keccak256("Transfer(address,address,uint256)")));
    assert_eq!(tx_receipt.logs[0].topics[1], H256::zero());
    assert_eq!(tx_receipt.logs[0].topics[2], H160::from(katana.eoa().evm_address().unwrap().as_fixed_bytes()).into());
    assert_eq!(
        tx_receipt.logs[0].data,
        ethers::types::Bytes::from_hex("0x0000000000000000000000000000000000000000000000000000000000000064").unwrap()
    );

    // eth_getTransactionByHash
    let tx = client.get_transaction(tx_receipt.transaction_hash).await.unwrap().unwrap();
    assert_eq!(tx.block_number, Some(block_number));
    assert_eq!(tx.from, katana.eoa().evm_address().unwrap().into());
    assert_eq!(tx.to, Some(contract.address()));
    assert_eq!(tx.value, 0u64.into());
    assert_eq!(tx.gas, 100.into());
    // Gas Price is None in TxType == 2, i.e. EIP1559
    assert_eq!(tx.gas_price, None);
    assert_eq!(tx.transaction_type, Some(2.into()));
    // TODO: Fix inconsistent max_fee_per_gas and max_priority_fee_per_gas
    assert_eq!(tx.max_fee_per_gas, Some(3000000002_u64.into()));
    assert_eq!(tx.max_priority_fee_per_gas, Some(0.into()));
    // ⚠️ Do not use Transaction::hash() to compare hashes
    // As it computes the keccak256 of the RLP encoding of the transaction
    // This is not the same as the transaction hash returned by the RPC (Starknet transaction hash)
    assert_eq!(tx.hash, tx_receipt.transaction_hash);

    // eth_getBlockByNumber
    let block = client.get_block(BlockId::Number(BlockNumber::Number(block_number))).await.unwrap().unwrap();
    assert_eq!(&block.number.unwrap(), &block_number);
    // Check that our transaction is inside the block
    assert_eq!(block.transactions.len(), 1);
    assert_eq!(block.transactions[0], tx_receipt.transaction_hash);

    // eth_syncing
    // TODO: Fix eth_syncing
    // let syncing = client.syncing().await.unwrap();
    // assert_eq!(syncing, SyncingStatus::IsFalse);
    // returns an error `MiddlewareError(JsonRpcClientError(JsonRpcError(JsonRpcError {
    // code: 0, message: "got code -32601 with: Method not found", data: None })))`

    // eth_gasPrice
    let gas_price = client.get_gas_price().await.unwrap();
    assert_eq!(gas_price, 1u64.into());

    // Stop the server
    server_handle.stop().expect("Failed to stop the server");
}
