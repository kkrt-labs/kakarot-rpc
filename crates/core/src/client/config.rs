use eyre::Result;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcTransport};
use starknet::providers::JsonRpcClient;
use url::Url;

use super::constants::{KATANA_RPC_URL, MADARA_RPC_URL};
use super::errors::ConfigError;

fn get_env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::EnvironmentVariableMissing(name.into()))
}

#[derive(Default, Clone)]
pub enum Network {
    #[default]
    Katana,
    Madara,
    Mainnet,
    Goerli1,
    Goerli2,
    Mock,
}

#[derive(Default, Clone)]
/// Configuration for the Starknet RPC client.
pub struct StarknetConfig {
    /// Rpc url. Optional as it can be provided by the environment variable `STARKNET_NETWORK`.
    pub url: Option<Url>,
    /// Kakarot contract address.
    pub kakarot_address: FieldElement,
    /// Proxy account class hash.
    pub proxy_account_class_hash: FieldElement,
    /// Additional configuration if the underlying provider is a Sequencer provider.
    pub network: Network,
}

impl StarknetConfig {
    pub fn new(
        url: Option<Url>,
        kakarot_address: FieldElement,
        proxy_account_class_hash: FieldElement,
        network: Network,
    ) -> Self {
        StarknetConfig { url, kakarot_address, proxy_account_class_hash, network }
    }

    /// Create a new `StarknetConfig` from environment variables.
    pub fn from_env() -> Result<Self, ConfigError> {
        let mut provider_url = None;
        let starknet_rpc = get_env_var("STARKNET_NETWORK")?;
        let network = match starknet_rpc.to_lowercase().as_str() {
            // TODO: Add possibility to set url for katana and madara in env rather than constants.
            "katana" => {
                provider_url = Some(Url::parse(KATANA_RPC_URL)?);
                Network::Katana
            }
            "madara" => {
                provider_url = Some(Url::parse(MADARA_RPC_URL)?);
                Network::Madara
            }
            // TODO: Add possibility to override gateway url for mainnet and testnet.
            "mainnet" => Network::Mainnet,
            "goerli1" => Network::Goerli1,
            "goerli2" => Network::Goerli2,
            "testnet" => Network::Goerli1,
            _ => Err(ConfigError::EnvironmentVariableSetWrong(format!(
                "STARKNET_NETWORK should be either katana, madara, goerli1, goerli2, testnet or mainnet got \
                 {starknet_rpc}"
            )))?,
        };

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

        Ok(StarknetConfig::new(provider_url, kakarot_address, proxy_account_class_hash, network))
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
    ///
    /// # Example
    ///
    /// ```rust
    /// use kakarot_rpc_core::client::config::{JsonRpcClientBuilder, Network, StarknetConfig};
    /// use starknet::core::types::FieldElement;
    /// use starknet::providers::jsonrpc::HttpTransport;
    /// use starknet::providers::JsonRpcClient;
    /// use url::Url;
    ///
    /// let url = "http://0.0.0.0:1234/rpc";
    /// let config = StarknetConfig::new(
    ///     Some(Url::parse(url).unwrap()),
    ///     FieldElement::default(),
    ///     FieldElement::default(),
    ///     Network::Katana,
    /// );
    /// let provider: JsonRpcClient<HttpTransport> =
    ///     JsonRpcClientBuilder::with_http(&config).unwrap().build();
    /// ```
    pub fn with_http(config: &StarknetConfig) -> Result<Self> {
        let url = config.clone().url.ok_or(eyre::eyre!("No url provided"))?;
        let transport = HttpTransport::new(url);
        Ok(Self::new(transport))
    }
}
