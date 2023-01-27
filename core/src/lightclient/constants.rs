// constants.rs
pub const ACCOUNT_REGISTRY_ADDRESS: &str =
    "0x052a419fd88f53f9a29d22c3d8db24dd9a9a01a41a483ac660d88622f83c40db";
pub const CHAIN_ID: u64 = 1263227476;

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const BYTECODE: FieldElement = selector!("bytecode");
}
