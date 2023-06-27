use eyre::Result;
use starknet::core::types::FieldElement;

use super::errors::ConfigError;

fn get_env_var(name: &str) -> Result<String, ConfigError> {
    std::env::var(name).map_err(|_| ConfigError::EnvironmentVariableMissing(name.into()))
}

pub struct StarknetConfig {
    pub starknet_rpc: String,
    pub kakarot_address: FieldElement,
    pub proxy_account_class_hash: FieldElement,
}

impl StarknetConfig {
    pub fn new(starknet_rpc: &str, kakarot_address: FieldElement, proxy_account_class_hash: FieldElement) -> Self {
        StarknetConfig { starknet_rpc: String::from(starknet_rpc), kakarot_address, proxy_account_class_hash }
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        let starknet_rpc_url = get_env_var("STARKNET_RPC_URL")?;

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

        Ok(StarknetConfig::new(&starknet_rpc_url, kakarot_address, proxy_account_class_hash))
    }
}
