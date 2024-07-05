use eyre::eyre;
use starknet::core::types::FieldElement;
use std::env::var;
use url::Url;

fn env_var_to_field_element(var_name: &str) -> Result<FieldElement, eyre::Error> {
    let env_var = var(var_name).map_err(|_| eyre!("missing env var: {var_name}"))?;

    Ok(FieldElement::from_hex_be(&env_var)?)
}

#[derive(Clone, Debug)]
/// Configuration for the Starknet RPC client.
pub struct KakarotRpcConfig {
    /// Starknet network.
    pub network_url: Url,
    /// Kakarot contract address.
    pub kakarot_address: FieldElement,
    /// Uninitialized account class hash.
    pub uninitialized_account_class_hash: FieldElement,
    /// Account contract class hash.
    pub account_contract_class_hash: FieldElement,
}

impl KakarotRpcConfig {
    /// `STARKNET_NETWORK` environment variable should be set the URL of a `JsonRpc`
    /// starknet provider, e.g. <https://starknet-goerli.g.alchemy.com/v2/some_key>.
    pub fn from_env() -> eyre::Result<Self> {
        Ok(Self {
            network_url: Url::parse(&var("STARKNET_NETWORK")?)?,
            kakarot_address: env_var_to_field_element("KAKAROT_ADDRESS")?,
            uninitialized_account_class_hash: env_var_to_field_element("UNINITIALIZED_ACCOUNT_CLASS_HASH")?,
            account_contract_class_hash: env_var_to_field_element("ACCOUNT_CONTRACT_CLASS_HASH")?,
        })
    }
}
