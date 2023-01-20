// constants.rs
pub const ACCOUNT_REGISTRY_ADDRESS: &str = "0x1234567890abcdef";

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const SELECTOR_GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const SELECTOR_BYTECODE: FieldElement = selector!("bytecode");
}
