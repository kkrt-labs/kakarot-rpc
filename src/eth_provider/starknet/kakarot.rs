use dotenv::dotenv;
use lazy_static::lazy_static;
use starknet_abigen_macros::abigen_legacy;
use starknet_abigen_parser;
use starknet_crypto::FieldElement;

fn env_var_to_field_element(var_name: &str) -> FieldElement {
    dotenv().ok();
    let env_var = std::env::var(var_name).unwrap_or_else(|_| panic!("Missing environment variable {var_name}"));

    FieldElement::from_hex_be(&env_var).unwrap_or_else(|_| panic!("Invalid hex string for {var_name}"))
}

lazy_static! {
    // Contract addresses
    pub static ref KAKAROT_ADDRESS: FieldElement = env_var_to_field_element("KAKAROT_ADDRESS");
    pub static ref STARKNET_NATIVE_TOKEN: FieldElement = FieldElement::from_hex_be("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7").unwrap();

    // Contract class hashes
    pub static ref PROXY_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("PROXY_ACCOUNT_CLASS_HASH");
    pub static ref EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH: FieldElement =
        env_var_to_field_element("EXTERNALLY_OWNED_ACCOUNT_CLASS_HASH");
    pub static ref CONTRACT_ACCOUNT_CLASS_HASH: FieldElement = env_var_to_field_element("CONTRACT_ACCOUNT_CLASS_HASH");
}

pub mod erc20 {
    use super::*;
    abigen_legacy!(ERC20, "./artifacts/fixtures/ERC20.json");
}

pub mod contract_account {
    use super::*;
    abigen_legacy!(ContractAccount, "./artifacts/contract_account.json");
}
