#[cfg(test)]
mod test {
    use kakarot_rpc::eth_rpc::KakarotEthRpc;
    use kakarot_rpc_core::client::MockStarknetClient;
    use reth_primitives::{Address, Bytes};

    use starknet::providers::jsonrpc::models::BlockId as StarknetBlockId;

    #[tokio::test]
    async fn test_eth_chain_id_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_chainId\", \"params\": [] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _has_stop = server_handle.stop().unwrap();

        // check rpc response match our needs
        // for this request check hexa is KKRT
    }

    #[tokio::test]
    async fn test_get_block_number_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_blockNumber\", \"params\": [] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _has_stop = server_handle.stop().unwrap();

        // response should return a fixed block id for the moment eg. 19640
    }

    #[tokio::test]
    async fn test_get_code_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getCode\", \"params\": [\"0xabde1007dcf45cb509ddde375162399a99880064\", \"latest\"] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _has_stop = server_handle.stop().unwrap();

        // response should return a fixed block id for the moment eg. 19640
    }

    #[tokio::test]
    async fn test_block_by_number_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByNumber\", \"params\": [] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _has_stop = server_handle.stop().unwrap();

        // response should return a fixed block id for the moment eg. 19640
    }

    #[tokio::test]
    async fn test_block_by_hash_is_ok() {
        let (_, server_handle) = setup_rpc_server().await;
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:3030")
            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_getBlockByHash\", \"params\": [] }")
            .header("content-type", "application/json")
            .send()
            .await
            .unwrap();

        let _has_stop = server_handle.stop().unwrap();

        // response should return a fixed block id for the moment eg. 19640
    }
}
