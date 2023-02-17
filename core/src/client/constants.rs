pub const KAKAROT_MAIN_CONTRACT_ADDRESS: &str =
    "0x566864dbc2ae76c2d12a8a5a334913d0806f85b7a4dccea87467c3ba3616e75";
pub const KAKAROT_CONTRACT_ACCOUNT_CLASS_HASH: &str =
    "0x0775033b738dfe34c48f43a839c3d882ebe521befb3447240f2d218f14816ef5";

pub const CHAIN_ID: u64 = 1263227476;

pub const STARKNET_NATIVE_TOKEN: &str =
    "2087021424722619777119509474943472645767659996348769578120564519014510906823";

pub mod selectors {
    use starknet::core::types::FieldElement;
    use starknet::macros::selector;

    pub const GET_STARKNET_CONTRACT_ADDRESS: FieldElement =
        selector!("get_starknet_contract_address");
    pub const BYTECODE: FieldElement = selector!("bytecode");

    pub const EXECUTE_AT_ADDRESS: FieldElement = selector!("execute_at_address");
    pub const COMPUTE_STARKNET_ADDRESS: FieldElement = selector!("compute_starknet_address");
    pub const CHAIN_ID: u64 = 1263227476_u64;

    pub const GET_EVM_ADDRESS: FieldElement = selector!("get_evm_address");

    pub const BALANCE_OF: FieldElement = selector!("balanceOf");
}
