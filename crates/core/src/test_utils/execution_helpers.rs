use reth_primitives::{BlockId, H256, U256};

use super::deploy_helpers::{create_raw_ethereum_tx, KakarotTestEnvironment};
use crate::client::api::KakarotEthApi;
use crate::models::felt::Felt252Wrapper;

pub async fn execute_tx(env: &KakarotTestEnvironment, contract: &str, selector: &str, args: Vec<U256>) -> H256 {
    let contract = env.evm_contract(contract);
    let contract_eth_address = {
        let address: Felt252Wrapper = contract.addresses.eth_address.into();
        address.try_into().unwrap()
    };
    let client = env.client();
    let kakarot = env.kakarot();

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
