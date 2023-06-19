#[cfg(test)]
mod tests {

    use kakarot_rpc_core::{
        client::{
            client_api::KakarotClient, convertible::ConvertibleStarknetBlock, models::BlockWithTxs,
        },
        mock::wiremock_utils::setup_mock_client_crate,
    };
    use reth_primitives::H256;
    use starknet::{
        core::types::{BlockId, BlockTag, FieldElement},
        providers::Provider,
    };

    #[tokio::test]
    async fn test_starknet_block_to_eth_block() {
        let client = setup_mock_client_crate().await;
        let starknet_client = client.inner();
        let starknet_block = starknet_client
            .get_block_with_txs(BlockId::Tag(BlockTag::Latest))
            .await
            .unwrap();
        let eth_block = BlockWithTxs::new(starknet_block)
            .to_eth_block(&client)
            .await
            .unwrap();

        // TODO: Add more assertions & refactor into assert helpers
        // assert helpers should allow import of fixture file
        assert_eq!(
            eth_block.header.hash,
            Some(H256::from_slice(
                &FieldElement::from_hex_be(
                    "0x449aa33ad836b65b10fa60082de99e24ac876ee2fd93e723a99190a530af0a9"
                )
                .unwrap()
                .to_bytes_be()
            ))
        )
    }
}
