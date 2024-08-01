use super::katana::Katana;
use crate::eth_rpc::{config::RPCConfig, rpc::KakarotRpcModuleBuilder, run_server};
use crate::alchemy_provider::provider::AlchemyStruct;
use jsonrpsee::server::ServerHandle;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{net::SocketAddr, sync::LazyLock};
use tokio::sync::Mutex;

/// A lazy static mutex for managing the next available port number.
///
/// It ensures thread-safe access to the next port number.
pub static NEXT_PORT: LazyLock<Mutex<u16>> = LazyLock::new(|| Mutex::new(3030));

/// Asynchronously gets the next available port number.
///
/// This function locks the `NEXT_PORT` mutex to ensure exclusive access
/// to the next port number and increments it for subsequent calls.
async fn get_next_port() -> u16 {
    let mut port = NEXT_PORT.lock().await;
    let next_port = *port;
    *port += 1;
    next_port
}

/// Sets up the environment for Kakarot RPC integration tests by deploying the Kakarot contracts
/// and starting the Kakarot RPC server.
///
/// This function:
/// 1. Takes an `Arc<TestSequencer>` as input, which is used to deploy the Kakarot contracts and to
///    set up the Kakarot RPC server.
/// 2. Deploys the Kakarot contracts.
/// 3. Creates Starknet and Kakarot clients.
/// 4. Sets up and runs the Kakarot RPC module.
///
/// # Arguments
///
/// * `starknet_test_sequencer` - An Arc-wrapped `TestSequencer`. This is used to deploy the Kakarot
///   contracts and to set up the Kakarot RPC server.
///
/// # Returns
///
/// This function returns a Result containing a tuple with the server's address and a handle to
/// stop the server upon successful execution.
///
/// The function may return an Err variant of `eyre::Report` if there are issues with deploying the
/// Kakarot contracts, creating the clients, or running the RPC server.
///
/// # Example
/// ```ignore
/// use kakarot_rpc::test_utils::start_kakarot_rpc_server;
/// use kakarot_rpc_core::test_utils::fixtures::kakarot_test_env_ctx;
/// use kakarot_rpc_core::test_utils::deploy_helpers::KakarotTestEnvironmentContext;
/// use dojo_test_utils::sequencer::TestSequencer;
/// use std::sync::Arc;
/// use tokio::runtime::Runtime;
/// use rstest::*;
///
/// #[rstest]
/// #[tokio::test]
/// async fn test_case(kakarot_test_env_ctx: KakarotTestEnvironmentContext) {
///    // Set up the Kakarot RPC integration environment.
///    let (server_addr, server_handle) = start_kakarot_rpc_server(&kakarot_test_env_ctx).await.unwrap();
///
///    // Query whatever eth_rpc endpoints
///
///    // Dont forget to close server at the end.
///    server_handle.stop().expect("Failed to stop the server");
///
/// }
/// ```
///
/// `allow(dead_code)` is used because this function is used in tests,
/// and each test is compiled separately, so the compiler thinks this function is unused
#[allow(dead_code)]
pub async fn start_kakarot_rpc_server(katana: &Katana) -> Result<(SocketAddr, ServerHandle), eyre::Report> {
    let alchemy_provider = AlchemyStruct::new(katana.eth_provider().clone());
    Ok(run_server(
        KakarotRpcModuleBuilder::new(katana.eth_provider(), katana.starknet_provider(), alchemy_provider).rpc_module()?,
        #[cfg(feature = "testing")]
        RPCConfig::new_test_config_from_port(get_next_port().await),
        #[cfg(not(feature = "testing"))]
        RPCConfig::from_port(get_next_port().await),
    )
    .await?)
}

/// Represents a builder for creating JSON-RPC requests.
/// Taken from <https://github.com/paradigmxyz/reth/blob/main/crates/rpc/rpc-builder/tests/it/http.rs>
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct RawRpcParamsBuilder {
    method: String,
    params: Vec<Value>,
    id: i32,
}

impl RawRpcParamsBuilder {
    /// Sets the method name for the JSON-RPC request.
    pub fn new(method: impl Into<String>) -> Self {
        Self { method: method.into(), params: Vec::new(), id: 1 }
    }

    /// Adds a parameter to the JSON-RPC request.
    #[must_use]
    pub fn add_param<S: Serialize>(mut self, param: S) -> Self {
        self.params.push(serde_json::to_value(param).expect("Failed to serialize parameter"));
        self
    }

    /// Sets the ID for the JSON-RPC request.
    #[must_use]
    pub const fn set_id(mut self, id: i32) -> Self {
        self.id = id;
        self
    }

    /// Constructs the JSON-RPC request string based on the provided configurations.

    pub fn build(self) -> String {
        let Self { method, params, id } = self;

        format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"{}","params":[{}]}}"#,
            id,
            method,
            params.into_iter().map(|p| p.to_string()).collect::<Vec<String>>().join(",")
        )
    }
}
