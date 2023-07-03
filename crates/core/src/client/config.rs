use eyre::Result;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcTransport};
use starknet::providers::JsonRpcClient;
use url::Url;

use super::errors::ConfigError;

fn get_env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::EnvironmentVariableMissing(name.into()))
}

#[derive(Default)]
/// Configuration for the Starknet RPC client.
pub struct StarknetConfig {
    /// Rpc url.
    pub url: String,
    /// Kakarot contract address.
    pub kakarot_address: FieldElement,
    /// Proxy account class hash.
    pub proxy_account_class_hash: FieldElement,
}

impl StarknetConfig {
    pub fn new(url: String, kakarot_address: FieldElement, proxy_account_class_hash: FieldElement) -> Self {
        StarknetConfig { url, kakarot_address, proxy_account_class_hash }
    }

    /// Create a new `StarknetConfig` from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let starknet_rpc = get_env_var("STARKNET_RPC_URL")?;

        let kakarot_address = get_env_var("KAKAROT_ADDRESS")?;
        let kakarot_address = FieldElement::from_hex_be(&kakarot_address).map_err(|_| {
            ConfigError::EnvironmentVariableSetWrong(format!(
                "KAKAROT_ADDRESS should be provided as a hex string, got {kakarot_address}"
            ))
        })?;

        let proxy_account_class_hash = get_env_var("PROXY_ACCOUNT_CLASS_HASH")?;
        let proxy_account_class_hash = FieldElement::from_hex_be(&proxy_account_class_hash).map_err(|_| {
            ConfigError::EnvironmentVariableSetWrong(format!(
                "PROXY_ACCOUNT_CLASS_HASH should be provided as a hex string, got {proxy_account_class_hash}"
            ))
        })?;

        Ok(StarknetConfig::new(starknet_rpc, kakarot_address, proxy_account_class_hash))
    }
}

/// A builder for a `JsonRpcClient`.
pub struct JsonRpcClientBuilder<T: JsonRpcTransport>(JsonRpcClient<T>);

impl<T: JsonRpcTransport> JsonRpcClientBuilder<T> {
    /// Create a new `JsonRpcClientBuilder`.
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport to use.
    pub fn new(transport: T) -> Self {
        Self(JsonRpcClient::new(transport))
    }

    /// Build the `JsonRpcClient`.
    pub fn build(self) -> JsonRpcClient<T> {
        self.0
    }
}

impl JsonRpcClientBuilder<HttpTransport> {
    /// Returns a new `JsonRpcClientBuilder` with a `HttpTransport`.
    pub fn with_http(config: &StarknetConfig) -> Result<Self> {
        let url = Url::parse(&config.url)?;
        let transport = HttpTransport::new(url);
        Ok(Self::new(transport))
    }
}
