use std::sync::Arc;

use eyre::Result;
use starknet::accounts::SingleOwnerAccount;
use starknet::core::types::FieldElement;
use starknet::providers::jsonrpc::{HttpTransport, JsonRpcTransport};
use starknet::providers::{JsonRpcClient, Provider, SequencerGatewayProvider};
use starknet::signers::{LocalWallet, SigningKey};
use url::Url;

use super::constants::{KATANA_RPC_URL, MADARA_RPC_URL};
use super::errors::ConfigError;

fn get_env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::EnvironmentVariableMissing(name.into()))
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
            Network::MainnetGateway => Ok(Url::parse("https://alpha-mainnet.starknet.io/feeder_gateway/")?),
            Network::Goerli1Gateway => Ok(Url::parse("https://alpha4.starknet.io/feeder_gateway/")?),
            Network::Goerli2Gateway => Ok(Url::parse("https://alpha4-2.starknet.io/feeder_gateway/")?),
            _ => Err(ConfigError::InvalidNetwork(format!("Network {:?} is not supported for gateway url", self))),
        }
    }

    pub fn provider_url(&self) -> Result<Url, ConfigError> {
        match self {
            Network::Katana => Ok(Url::parse(KATANA_RPC_URL)?),
            Network::Madara => Ok(Url::parse(MADARA_RPC_URL)?),
            Network::Sharingan => Ok(Url::parse(
                std::env::var("SHARINGAN_RPC_URL")
                    .map_err(|_| ConfigError::EnvironmentVariableMissing("SHARINGAN_RPC_URL".to_string()))?
                    .as_str(),
            )?),
            Network::JsonRpcProvider(url) => Ok(url.clone()),
            _ => Err(ConfigError::InvalidNetwork(format!("Network {:?} is not supported for provider url", self))),
        }
    }
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
    /// EOA class hash.
    pub externally_owned_account_class_hash: FieldElement,
    /// Contract Account class hash.
    pub contract_account_class_hash: FieldElement,
}

impl StarknetConfig {
    pub fn new(
        network: Network,
        kakarot_address: FieldElement,
        proxy_account_class_hash: FieldElement,
        externally_owned_account_class_hash: FieldElement,
        contract_account_class_hash: FieldElement,
    ) -> Self {
        StarknetConfig {
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
        let network = get_env_var("STARKNET_NETWORK")?;
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

        let externally_owned_account_class_hash = get_env_var("EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH")?;
        let externally_owned_account_class_hash = FieldElement::from_hex_be(&externally_owned_account_class_hash)
            .map_err(|_| {
                ConfigError::EnvironmentVariableSetWrong(format!(
                    "EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH should be provided as a hex string, got \
                     {externally_owned_account_class_hash}"
                ))
            })?;

        let contract_account_class_hash = get_env_var("CONTRACT_ACCOUNT_CLASS_HASH")?;
        let contract_account_class_hash = FieldElement::from_hex_be(&contract_account_class_hash).map_err(|_| {
            ConfigError::EnvironmentVariableSetWrong(format!(
                "CONTRACT_ACCOUNT_CLASS_HASH should be provided as a hex string, got {contract_account_class_hash}"
            ))
        })?;

        Ok(StarknetConfig::new(
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

    /// Build the `SequencerGatewayProvider`.
    pub fn build(self) -> SequencerGatewayProvider {
        self.0
    }
}

pub async fn get_starknet_account_from_env<P: Provider + Send + Sync + 'static>(
    provider: Arc<P>,
) -> Result<SingleOwnerAccount<Arc<P>, LocalWallet>> {
    let (starknet_account_private_key, starknet_account_address) = {
        let starknet_account_private_key = get_env_var("DEPLOYER_ACCOUNT_PRIVATE_KEY")?;
        let starknet_account_private_key = FieldElement::from_hex_be(&starknet_account_private_key).map_err(|_| {
            ConfigError::EnvironmentVariableSetWrong(format!(
                "DEPLOYER_ACCOUNT_PRIVATE_KEY should be provided as a hex string, got {starknet_account_private_key}"
            ))
        })?;

        let starknet_account_address = get_env_var("DEPLOYER_ACCOUNT_ADDRESS")?;
        let starknet_account_address = FieldElement::from_hex_be(&starknet_account_address).map_err(|_| {
            ConfigError::EnvironmentVariableSetWrong(format!(
                "DEPLOYER_ACCOUNT_ADDRESS should be provided as a hex string, got {starknet_account_private_key}"
            ))
        })?;
        (starknet_account_private_key, starknet_account_address)
    };

    let chain_id = provider.chain_id().await?;

    let local_wallet = LocalWallet::from_signing_key(SigningKey::from_secret_scalar(starknet_account_private_key));
    Ok(SingleOwnerAccount::new(provider.clone(), local_wallet, starknet_account_address, chain_id))
}
