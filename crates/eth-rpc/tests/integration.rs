mod utils;

#[cfg(test)]
mod integration_tests {
    use std::str::FromStr;

    use ethers::prelude::{Block as EthersBlock, Http as EthersHttp, H256 as EthersH256};
    use reth_primitives::U64;

    use crate::utils::setup_kakarot_rpc_integration_env;

    #[tokio::test]
    async fn test_get_block_rpc() {
        // Setup the test sequencer, deploy the Kakarot system and start the RPC server
        let (server_addr, server_handle, _sequencer) = setup_kakarot_rpc_integration_env()
            .await
            .map_err(|e| {
                println!("Error setting up Kakarot RPC server: {}", e);
                e
            })
            .unwrap();

        // try to run the test
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
