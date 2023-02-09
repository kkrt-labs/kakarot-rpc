// //! Kakarot RPC module for Ethereum.
// //! It is an adapter layer to interact with Kakarot ZK-EVM.
use std::net::{AddrParseError, SocketAddr};
pub mod eth_rpc;
use eth_rpc::{EthApiServer, KakarotEthRpc};
use eyre::Result;
use jsonrpsee::server::{ServerBuilder, ServerHandle};
use kakarot_rpc_core::client::StarknetClient;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RpcError {
    #[error(transparent)]
    JsonRpcServerError(#[from] jsonrpsee::core::Error),
    #[error(transparent)]
    ParseError(#[from] AddrParseError),
}

pub async fn run_server(
    starknet_client: Box<dyn StarknetClient>,
) -> Result<(SocketAddr, ServerHandle), RpcError> {
    let server = ServerBuilder::default()
        .build("127.0.0.1:3030".parse::<SocketAddr>()?)
        .await?;

    let addr = server.local_addr()?;

    let rpc_calls = KakarotEthRpc::new(starknet_client);
    let handle = server.start(rpc_calls.into_rpc())?;

    Ok((addr, handle))
}

pub mod test_utils {
    use jsonrpsee::server::ServerHandle;
    use kakarot_rpc_core::{client::StarknetClientImpl, utils::wiremock_utils::setup_wiremock};

    use crate::run_server;

    /// Run wiremock to fake starknet rpc and then run our own kakarot_rpc_server.
    ///
    /// Example :
    /// ```
    ///   use kakarot_rpc::test_utils::setup_rpc_server;
    ///
    ///   #[tokio::test]
    ///   async fn test_case() {
    ///       // Run base server
    ///       let (_, server_handle) = setup_rpc_server().await;
    ///
    ///       //Query whatever eth_rpc endpoints
    ///       let client = reqwest::Client::new();
    ///        let res = client
    ///            .post("http://127.0.0.1:3030")
    ///            .body("{\"jsonrpc\": \"2.0\", \"id\": 1, \"method\": \"eth_chainId\", \"params\": [] }")
    ///            .header("content-type", "application/json")
    ///            .send()
    ///            .await
    ///            .unwrap();
    ///
    ///        // Dont forget to close server at the end.
    ///        let _has_stop = server_handle.stop().unwrap();
    ///   }
    /// ```
    pub async fn setup_rpc_server() -> (String, ServerHandle) {
        let starknet_rpc = setup_wiremock().await;

        let starknet_lightclient = StarknetClientImpl::new(&starknet_rpc).unwrap();
        let (_rpc_server_uri, server_handle) =
            run_server(Box::new(starknet_lightclient)).await.unwrap();
        (starknet_rpc, server_handle)
    }
}
