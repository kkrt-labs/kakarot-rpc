#[cfg(test)]
mod tests {

    use kakarot_rpc::test_utils::setup_rpc_server;

    use kakarot_rpc_core::utils::wiremock_utils::EthJsonRpcResponse;

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

        let block_hash = res.json::<EthJsonRpcResponse<String>>().await.unwrap();
        // see rpc/src/eth_rpc.rs:286 to get ASCII String
        assert_eq!(block_hash.result, String::from("0x4b4b5254"));

        server_handle.stop().unwrap();
    }
}
