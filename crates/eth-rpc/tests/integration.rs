mod utils;

#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use ethers::prelude::{Block as EthersBlock, Http as EthersHttp, H256 as EthersH256};
    use reth_primitives::U64;

    use crate::utils::setup_kakarot_rpc_integration_env;

    #[tokio::test]
    #[ignore]
    async fn test_get_block_rpc() {
        // initialize the
        let (server_addr, server_handle) = setup_kakarot_rpc_integration_env()
            .await
            .map_err(|e| {
                println!("Error setting up Kakarot RPC server: {}", e);
                e
            })
            .unwrap();

        // wait for the server to start
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // try to run the test
        let provider = EthersHttp::from_str(format!("http://localhost:{}", server_addr.port()).as_ref()).unwrap();
        let block_number: U64 = ethers::prelude::JsonRpcClient::request(&provider, "eth_blockNumber", ())
            .await
            .expect("Failed to get block number");

        let params = (block_number, true);
        let block: std::result::Result<EthersBlock<EthersH256>, _> =
            ethers::prelude::JsonRpcClient::request(&provider, "eth_getBlockByNumber", params).await;
        assert!(block.is_ok());
        server_handle.stopped().await;
    }
}
