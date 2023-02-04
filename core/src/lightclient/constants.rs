pub const KAKAROT_MAIN_CONTRACT_ADDRESS: &str =
    "0x3514d37b24aa44df0b932ee380aa116e9fdace78ac23c28fa1a209479445a66";

pub const CHAIN_ID: u64 = 1263227476;

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const BYTECODE: FieldElement = selector!("bytecode");

    pub const EXECUTE_AT_ADDRESS: FieldElement = selector!("execute_at_address");

    pub const COMPUTE_STARKNET_ADDRESS: FieldElement = selector!("compute_starknet_address");
}
