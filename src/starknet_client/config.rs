use eyre::Result;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcTransport};
use starknet::providers::{JsonRpcClient, SequencerGatewayProvider};
use url::Url;

use super::constants::{KATANA_RPC_URL, MADARA_RPC_URL};
use super::errors::ConfigError;

fn env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::EnvironmentVariableMissing(name.into()))
}

fn field_element_from_env(var_name: &str) -> Result<FieldElement, ConfigError> {
    let env_var = env_var(var_name)?;

    FieldElement::from_hex_be(&env_var)
        .map_err(|err| ConfigError::EnvironmentVariableSetWrong(var_name.into(), err.to_string()))
}

#[derive(Default, Clone, Debug)]
pub enum Network {
    #[default]
    Katana,
    Madara,
    Sharingan,
    MainnetGateway,
    Goerli1Gateway,
    Goerli2Gateway,
    JsonRpcProvider(Url),
}

impl Network {
    pub fn gateway_url(&self) -> Result<Url, ConfigError> {
        match self {
            Self::MainnetGateway => Ok(Url::parse("https://alpha-mainnet.starknet.io/feeder_gateway/")?),
            Self::Goerli1Gateway => Ok(Url::parse("https://alpha4.starknet.io/feeder_gateway/")?),
            Self::Goerli2Gateway => Ok(Url::parse("https://alpha4-2.starknet.io/feeder_gateway/")?),
            _ => Err(ConfigError::InvalidNetwork(format!("Network {:?} is not supported for gateway url", self))),
        }
    }

    pub fn provider_url(&self) -> Result<Url, ConfigError> {
        match self {
            Self::Katana => Ok(Url::parse(KATANA_RPC_URL)?),
            Self::Madara => Ok(Url::parse(MADARA_RPC_URL)?),
            Self::Sharingan => Ok(Url::parse(
                std::env::var("SHARINGAN_RPC_URL")
                    .map_err(|_| ConfigError::EnvironmentVariableMissing("SHARINGAN_RPC_URL".to_string()))?
                    .as_str(),
            )?),
            Self::JsonRpcProvider(url) => Ok(url.clone()),
            _ => Err(ConfigError::InvalidNetwork(format!("Network {:?} is not supported for provider url", self))),
        }
    }
}

#[derive(Default, Clone)]
/// Configuration for the Starknet RPC client.
pub struct KakarotRpcConfig {
    /// Starknet network.
    pub network: Network,
    /// Kakarot contract address.
    pub kakarot_address: FieldElement,
    /// Proxy account class hash.
    pub proxy_account_class_hash: FieldElement,
    /// Eoa class hash.
    pub externally_owned_account_class_hash: FieldElement,
    /// Contract Account class hash.
    pub contract_account_class_hash: FieldElement,
}

impl KakarotRpcConfig {
    pub const fn new(
        network: Network,
        kakarot_address: FieldElement,
        proxy_account_class_hash: FieldElement,
        externally_owned_account_class_hash: FieldElement,
        contract_account_class_hash: FieldElement,
    ) -> Self {
        Self {
            network,
            kakarot_address,
            proxy_account_class_hash,
            externally_owned_account_class_hash,
            contract_account_class_hash,
        }
    }

    /// Create a new `StarknetConfig` from environment variables.
    /// When using non-standard providers (i.e. not "katana", "madara", "mainnet"), the
    /// `STARKNET_NETWORK` environment variable should be set the URL of a JsonRpc
    /// starknet provider, e.g. https://starknet-goerli.g.alchemy.com/v2/some_key.
    pub fn from_env() -> Result<Self, ConfigError> {
        let network = env_var("STARKNET_NETWORK")?;
        let network = match network.to_lowercase().as_str() {
            "katana" => Network::Katana,
            "madara" => Network::Madara,
            "sharingan" => Network::Sharingan,
            "mainnet" => Network::MainnetGateway,
            "goerli1" => Network::Goerli1Gateway,
            "goerli2" => Network::Goerli2Gateway,
            "testnet" => Network::Goerli1Gateway,
            network_url => Network::JsonRpcProvider(Url::parse(network_url)?),
        };

        let kakarot_address = field_element_from_env("KAKAROT_ADDRESS")?;
        let proxy_account_class_hash = field_element_from_env("PROXY_ACCOUNT_CLASS_HASH")?;
        let externally_owned_account_class_hash = field_element_from_env("EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH")?;
        let contract_account_class_hash = field_element_from_env("CONTRACT_ACCOUNT_CLASS_HASH")?;

        Ok(Self::new(
            network,
            kakarot_address,
            proxy_account_class_hash,
            externally_owned_account_class_hash,
            contract_account_class_hash,
        ))
    }
}

/// A builder for a `JsonRpcClient`.
pub struct JsonRpcClientBuilder<T: JsonRpcTransport>(JsonRpcClient<T>);

impl<T: JsonRpcTransport> JsonRpcClientBuilder<T> {
    /// Create a new `JsonRpcClientBuilder`.
    pub fn new(transport: T) -> Self {
        Self(JsonRpcClient::new(transport))
    }

    // This clippy lint is false positive, trying to make this function `const` but it doesn't work.
    #[allow(clippy::missing_const_for_fn)]
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
    /// use kakarot_rpc::starknet_client::config::{JsonRpcClientBuilder, KakarotRpcConfig, Network};
    /// use starknet::core::types::FieldElement;
    /// use starknet::providers::jsonrpc::HttpTransport;
    /// use starknet::providers::JsonRpcClient;
    /// use url::Url;
    ///
    /// let url = "http://0.0.0.0:1234/rpc";
    /// let config = KakarotRpcConfig::new(
    ///     Network::JsonRpcProvider(Url::parse(url).unwrap()),
    ///     FieldElement::default(),
    ///     FieldElement::default(),
    ///     FieldElement::default(),
    ///     FieldElement::default(),
    /// );
    /// let starknet_provider: JsonRpcClient<HttpTransport> =
    ///     JsonRpcClientBuilder::with_http(&config).unwrap().build();
    /// ```
    pub fn with_http(config: &KakarotRpcConfig) -> Result<Self> {
        let url = config.network.provider_url()?;
        let transport = HttpTransport::new(url);
        Ok(Self::new(transport))
    }
}

/// A builder for a `SequencerGatewayProvider`.
pub struct SequencerGatewayProviderBuilder(SequencerGatewayProvider);

impl SequencerGatewayProviderBuilder {
    /// Create a new `SequencerGatewayProviderBuilder`.
    pub fn new(network: &Network) -> Self {
        match network {
            Network::MainnetGateway => Self(SequencerGatewayProvider::starknet_alpha_mainnet()),
            Network::Goerli1Gateway => Self(SequencerGatewayProvider::starknet_alpha_goerli()),
            Network::Goerli2Gateway => Self(SequencerGatewayProvider::starknet_alpha_goerli_2()),
            _ => panic!("Unsupported network for SequencerGatewayProvider"),
        }
    }

    // This clippy lint is false positive, trying to make this function `const` but it doesn't work.
    #[allow(clippy::missing_const_for_fn)]
    /// Build the `SequencerGatewayProvider`.
    pub fn build(self) -> SequencerGatewayProvider {
        self.0
    }
}
