use reth_primitives::{BlockId, H256, U256};

use super::deploy_helpers::{create_raw_ethereum_tx, KakarotTestEnvironmentContext};
use crate::client::api::KakarotEthApi;

pub async fn execute_tx(env: &KakarotTestEnvironmentContext, contract: &str, selector: &str, args: Vec<U256>) -> H256 {
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
