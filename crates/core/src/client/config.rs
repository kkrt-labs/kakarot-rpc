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
    MainnetGateway,
    Goerli1Gateway,
    Goerli2Gateway,
    JsonRpcProvider(Url),
}

#[derive(Default, Clone)]
/// Configuration for the Starknet RPC client.
pub struct StarknetConfig {
    /// Starknet network.
    pub network: Network,
    /// Kakarot contract address.
    pub kakarot_address: FieldElement,
    /// Proxy account class hash.
    pub proxy_account_class_hash: FieldElement,
}

impl StarknetConfig {
    pub fn new(network: Network, kakarot_address: FieldElement, proxy_account_class_hash: FieldElement) -> Self {
        StarknetConfig { network, kakarot_address, proxy_account_class_hash }
    }

    /// Create a new `StarknetConfig` from environment variables.
    /// When using non-standard providers (i.e. not "katana", "madara", "mainnet"), the
    /// `STARKNET_NETWORK` environment variable should be set the URL of a JsonRpc
    /// starknet provider, e.g. https://starknet-goerli.g.alchemy.com/v2/some_key.
    pub fn from_env() -> Result<Self, ConfigError> {
        let network = get_env_var("STARKNET_NETWORK")?;
        let network = match network.to_lowercase().as_str() {
            "katana" => Network::Katana,
            "madara" => Network::Madara,
            "mainnet" => Network::MainnetGateway,
            "goerli1" => Network::Goerli1Gateway,
            "goerli2" => Network::Goerli2Gateway,
            "testnet" => Network::Goerli1Gateway,
            network_url => Network::JsonRpcProvider(Url::parse(network_url)?),
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

        Ok(StarknetConfig::new(network, kakarot_address, proxy_account_class_hash))
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
    /// Currently only supports Katana and Madara networks or manual Starknet provider URL.
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
    ///     Network::JsonRpcProvider(Url::parse(url).unwrap()),
    ///     FieldElement::default(),
    ///     FieldElement::default(),
    /// );
    /// let starknet_provider: JsonRpcClient<HttpTransport> =
    ///     JsonRpcClientBuilder::with_http(&config).unwrap().build();
    /// ```
    pub fn with_http(config: &StarknetConfig) -> Result<Self> {
        let url = match config.clone().network {
            Network::Katana => Url::parse(KATANA_RPC_URL)?,
            Network::Madara => Url::parse(MADARA_RPC_URL)?,
            Network::JsonRpcProvider(url) => url,
            _ => {
                return Err(eyre::eyre!(
                    "Constant networks (one of: [MainnetGateway, Goerli1Gateway, Goerli2Gateway, Mock]) is not \
                     supported"
                ));
            }
        };
        let transport = HttpTransport::new(url);
        Ok(Self::new(transport))
    }
}
