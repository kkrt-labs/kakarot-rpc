#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use dotenv::dotenv;
    use ethers::prelude::{Block as EthersBlock, Http as EthersHttp, H256 as EthersH256};
    use kakarot_rpc::test_utils::start_kakarot_rpc_server;
    use kakarot_test_utils::deploy_helpers::KakarotTestEnvironmentContext;
    use kakarot_test_utils::fixtures::kakarot_test_env_ctx;
    use reth_primitives::U64;
    use rstest::*;

    #[rstest]
    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_block_rpc(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
        // Load env
        dotenv().ok();

        // Start the Kakarot RPC server
        let (server_addr, server_handle) =
            start_kakarot_rpc_server(&kakarot_test_env_ctx).await.expect("Error setting up Kakarot RPC server");

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
