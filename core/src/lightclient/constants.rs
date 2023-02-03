pub const KAKAROT_MAIN_CONTRACT_ADDRESS: &str =
    "0x4615a6affcb60711b961585219a942a12539495e24443d280e1c73e443555b";

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
