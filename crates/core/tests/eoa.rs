use std::str::FromStr;

use ethers::signers::{LocalWallet, Signer};
use kakarot_rpc_core::client::constants::DEPLOY_FEE;
use kakarot_rpc_core::models::felt::Felt252Wrapper;
use kakarot_test_utils::deploy_helpers::KakarotTestEnvironmentContext;
use kakarot_test_utils::execution_helpers::execute_eth_transfer_tx;
use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
use reth_primitives::{Address, BlockId, BlockNumberOrTag, H256, U256};
use rstest::*;
use starknet::core::types::{BlockId as StarknetBlockId, BlockTag};

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_rpc_should_not_raise_when_eoa_not_deployed(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    // Given
    let client = kakarot_test_env_ctx.client();

    // When
    let nonce = client.nonce(Address::zero(), BlockId::from(BlockNumberOrTag::Latest)).await.unwrap();

    // Then
    // Zero address shouldn't throw 'ContractNotFound', but return zero
    assert_eq!(U256::from(0), nonce);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_check_eoa_account_exists(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    let (client, kakarot) = kakarot_test_env_ctx.resources();
    let block_id = StarknetBlockId::Tag(BlockTag::Latest);
    // this address shouldn't be shared with other tests, otherwise a test might deploy it in parallel,
    // and this test will fail; source -> ganache (https://github.com/trufflesuite/ganache)
    let evm_address_not_existing = Address::from_str("0xcE16e8eb8F4BF2E65BA9536C07E305b912BAFaCF").unwrap();

    let res = client.check_eoa_account_exists(kakarot.eoa_addresses.eth_address, &block_id).await.unwrap();
    assert!(res);

    let res = client.check_eoa_account_exists(evm_address_not_existing, &block_id).await.unwrap();
    assert!(!res)
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_deploy_eoa(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    let (client, kakarot) = kakarot_test_env_ctx.resources();
    let block_id = StarknetBlockId::Tag(BlockTag::Latest);
    // this address shouldn't be shared with other tests, otherwise a test might deploy it in parallel,
    // and this test will fail; source -> ganache (https://github.com/trufflesuite/ganache)
    let ethereum_address_to_deploy = Address::from_str("0x02f1c4C93AFEd946Cce5Ad7D34354A150bEfCFcF").unwrap();
    let amount: u128 = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

    // checking the account is not already deployed
    let res = client.check_eoa_account_exists(ethereum_address_to_deploy, &block_id).await.unwrap();
    assert!(!res);

    // funding account so it can cover its deployment fee
    let _ = execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, ethereum_address_to_deploy, amount)
        .await;

    let _ = client.deploy_eoa(ethereum_address_to_deploy).await.unwrap();

    // checking that the account is deployed
    let res = client.check_eoa_account_exists(ethereum_address_to_deploy, &block_id).await.unwrap();
    assert!(res);
}

#[rstest]
#[tokio::test(flavor = "multi_thread")]
async fn test_automatic_deployment_of_eoa(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
    let (_, kakarot) = kakarot_test_env_ctx.resources();

    // the private key has been taken from the ganache repo and can be safely published, do no share
    // with other tests https://github.com/trufflesuite/ganache
    let ethereum_private_key = "0x7f109a9e3b0d8ecfba9cc23a3614433ce0fa7ddcc80f2a8f10b222179a5a80d6";
    let to = LocalWallet::from_str(ethereum_private_key).unwrap();
    let to_private_key = {
        let signing_key_bytes = to.signer().to_bytes(); // Convert to bytes
        H256::from_slice(&signing_key_bytes) // Convert to H256
    };
    let to_address: Address = to.address().into();

    let deploy_fee: u128 = Felt252Wrapper::from(*DEPLOY_FEE).try_into().unwrap();

    let _ = execute_eth_transfer_tx(&kakarot_test_env_ctx, kakarot.eoa_private_key, to_address, deploy_fee * 2).await;

    let _ =
        execute_eth_transfer_tx(&kakarot_test_env_ctx, to_private_key, kakarot.eoa_addresses.eth_address, deploy_fee)
            .await;
}
