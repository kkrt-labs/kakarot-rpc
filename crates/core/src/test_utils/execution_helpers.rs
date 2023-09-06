use ethers::abi::Token;
use ethers::signers::{LocalWallet, Signer};
use reth_primitives::{Address, BlockId, H256};

use super::deploy_helpers::{create_eth_transfer_tx, create_raw_ethereum_tx, KakarotTestEnvironmentContext};
use crate::client::api::KakarotEthApi;

pub async fn execute_tx(env: &KakarotTestEnvironmentContext, contract: &str, selector: &str, args: Vec<Token>) -> H256 {
    let (client, kakarot, contract, contract_eth_address) = env.resources_with_contract(contract);

    // When
    let nonce = client
        .nonce(kakarot.eoa_addresses.eth_address, BlockId::Number(reth_primitives::BlockNumberOrTag::Latest))
        .await
        .unwrap();

    // Encode input, otherwise throw error
    let data = contract.abi.function(selector).unwrap().encode_input(&args).expect("Encoding error");

    let tx = create_raw_ethereum_tx(kakarot.eoa_private_key, contract_eth_address, data, nonce.try_into().unwrap());

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
