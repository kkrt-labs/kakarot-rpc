#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use ethers::prelude::{Block as EthersBlock, Http as EthersHttp, H256 as EthersH256};
    use kakarot_test_utils::fixtures::katana;
    use kakarot_test_utils::rpc::start_kakarot_rpc_server;
    use kakarot_test_utils::sequencer::Katana;
    use reth_primitives::U64;
    use rstest::*;

    #[rstest]
    #[awt]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_block_rpc(#[future] katana: Katana) {
        // Start the Kakarot RPC server
        let (server_addr, server_handle) =
            start_kakarot_rpc_server(&katana).await.expect("Error setting up Kakarot RPC server");

        // Run the test
        let provider = EthersHttp::from_str(format!("http://localhost:{}", server_addr.port()).as_ref()).unwrap();
        let block_number: U64 =
            ethers::prelude::JsonRpcClient::request(&provider, "eth_blockNumber", ()).await.unwrap();
        let params = (block_number, true);
        let block: std::result::Result<EthersBlock<EthersH256>, _> =
            ethers::prelude::JsonRpcClient::request(&provider, "eth_getBlockByNumber", params).await;
        assert!(block.is_ok());

        // Stop the server
        server_handle.stop().expect("Failed to stop the server");
    }
}
